use std::fs;
use std::io::{Cursor, IsTerminal, Write};
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use reqwest::Client;
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tar::Archive;
use tempfile::TempDir;

use crate::cli::UpdateArgs;
use crate::error::{Result, SkyTabError};

const REPO: &str = "LukasMurdock/skytab-cli";
const ARCHIVE_NAME: &str = "skytab";

#[derive(Debug, Clone, Serialize)]
pub struct UpdateReport {
    pub current_version: String,
    pub target_version: String,
    pub update_available: bool,
    pub updated: bool,
    pub check_only: bool,
    pub target_triple: String,
    pub installed_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

pub async fn run_update(args: UpdateArgs) -> Result<UpdateReport> {
    let client = Client::new();
    let current_version = format!("v{}", env!("CARGO_PKG_VERSION"));
    let target_triple = detect_target_triple()?;
    let explicit_version_requested = args.version.is_some();
    let target_version = match args.version {
        Some(tag) => normalize_tag(&tag),
        None => fetch_latest_tag(&client).await?,
    };

    let update_available = is_update_available(
        &current_version,
        &target_version,
        explicit_version_requested,
    );
    if args.check {
        return Ok(UpdateReport {
            current_version,
            target_version,
            update_available,
            updated: false,
            check_only: true,
            target_triple,
            installed_paths: Vec::new(),
        });
    }

    if !update_available {
        return Ok(UpdateReport {
            current_version,
            target_version,
            update_available,
            updated: false,
            check_only: false,
            target_triple,
            installed_paths: Vec::new(),
        });
    }

    if !args.yes && std::io::stdin().is_terminal() {
        let mut stdout = std::io::stdout();
        write!(
            stdout,
            "Update skytab from {current_version} to {target_version}? [y/N]: "
        )?;
        stdout.flush()?;

        let mut response = String::new();
        std::io::stdin().read_line(&mut response)?;
        let accepted = matches!(response.trim().to_ascii_lowercase().as_str(), "y" | "yes");
        if !accepted {
            return Ok(UpdateReport {
                current_version,
                target_version,
                update_available,
                updated: false,
                check_only: false,
                target_triple,
                installed_paths: Vec::new(),
            });
        }
    }

    let archive_asset = format!("{ARCHIVE_NAME}-{target_version}-{target_triple}.tar.gz");
    let checksums_url =
        format!("https://github.com/{REPO}/releases/download/{target_version}/checksums.txt");
    let archive_url =
        format!("https://github.com/{REPO}/releases/download/{target_version}/{archive_asset}");

    let checksums = client
        .get(&checksums_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let expected_checksum = parse_checksum_line(&checksums, &archive_asset)?;

    let archive_bytes = client
        .get(&archive_url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    let actual_checksum = sha256_hex(&archive_bytes);
    if actual_checksum != expected_checksum {
        return Err(SkyTabError::InvalidArgument(
            "checksum verification failed for update archive".into(),
        ));
    }

    let extracted = extract_archive(&archive_bytes)?;
    let mut installed_paths = Vec::new();

    let skytab_path = install_binary_from_extracted(&extracted, "skytab")?;
    installed_paths.push(skytab_path.display().to_string());

    if args.also_mcp {
        let current_exe = std::env::current_exe()?;
        if let Some(parent) = current_exe.parent() {
            let mcp_destination = parent.join("skytab-mcp");
            if mcp_destination.exists() {
                let mcp_path =
                    install_binary_from_extracted_to(&extracted, "skytab-mcp", &mcp_destination)?;
                installed_paths.push(mcp_path.display().to_string());
            }
        }
    }

    Ok(UpdateReport {
        current_version,
        target_version,
        update_available,
        updated: true,
        check_only: false,
        target_triple,
        installed_paths,
    })
}

async fn fetch_latest_tag(client: &Client) -> Result<String> {
    let response = client
        .get(format!(
            "https://api.github.com/repos/{REPO}/releases/latest"
        ))
        .header(reqwest::header::USER_AGENT, "skytab-cli")
        .send()
        .await?
        .error_for_status()?;

    let release: GitHubRelease = response.json().await?;
    Ok(normalize_tag(&release.tag_name))
}

fn detect_target_triple() -> Result<String> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok("aarch64-apple-darwin".to_string()),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin".to_string()),
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-musl".to_string()),
        (os, arch) => Err(SkyTabError::InvalidArgument(format!(
            "unsupported platform for self-update: {os}/{arch}"
        ))),
    }
}

fn normalize_tag(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.starts_with('v') {
        trimmed.to_string()
    } else {
        format!("v{trimmed}")
    }
}

fn is_update_available(
    current_version: &str,
    target_version: &str,
    explicit_version_requested: bool,
) -> bool {
    if explicit_version_requested {
        return current_version != target_version;
    }

    compare_versions(target_version, current_version)
        .map(|ordering| ordering.is_gt())
        .unwrap_or_else(|| current_version != target_version)
}

fn compare_versions(left: &str, right: &str) -> Option<std::cmp::Ordering> {
    let left = Version::parse(strip_v_prefix(left)).ok()?;
    let right = Version::parse(strip_v_prefix(right)).ok()?;
    Some(left.cmp(&right))
}

fn strip_v_prefix(value: &str) -> &str {
    value.strip_prefix('v').unwrap_or(value)
}

fn parse_checksum_line(content: &str, asset_name: &str) -> Result<String> {
    for line in content.lines() {
        let mut parts = line.split_whitespace();
        let sum = parts.next();
        let name = parts.next();
        if let (Some(sum), Some(name)) = (sum, name)
            && name == asset_name
        {
            return Ok(sum.to_string());
        }
    }

    Err(SkyTabError::InvalidArgument(format!(
        "checksum not found for asset {asset_name}"
    )))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn extract_archive(archive_bytes: &[u8]) -> Result<TempDir> {
    let temp_dir = tempfile::tempdir()?;
    let cursor = Cursor::new(archive_bytes);
    let decoder = GzDecoder::new(cursor);
    let mut archive = Archive::new(decoder);
    archive.unpack(temp_dir.path())?;
    Ok(temp_dir)
}

fn install_binary_from_extracted(extracted: &TempDir, binary_name: &str) -> Result<PathBuf> {
    let current_exe = std::env::current_exe()?;
    install_binary_from_extracted_to(extracted, binary_name, &current_exe)
}

fn install_binary_from_extracted_to(
    extracted: &TempDir,
    binary_name: &str,
    destination: &Path,
) -> Result<PathBuf> {
    let source_path = extracted.path().join(binary_name);
    if !source_path.exists() {
        return Err(SkyTabError::InvalidArgument(format!(
            "binary {binary_name} not found in archive"
        )));
    }

    let parent = destination.parent().ok_or_else(|| {
        SkyTabError::InvalidArgument("unable to resolve install directory for binary".into())
    })?;
    if !parent.exists() {
        return Err(SkyTabError::InvalidArgument(format!(
            "install directory does not exist: {}",
            parent.display()
        )));
    }

    let staging = parent.join(format!(".{binary_name}.new"));
    fs::copy(&source_path, &staging)?;

    let metadata = fs::metadata(&source_path)?;
    fs::set_permissions(&staging, metadata.permissions())?;

    fs::rename(&staging, destination)?;
    Ok(destination.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_tag_adds_prefix() {
        assert_eq!(normalize_tag("0.1.2"), "v0.1.2");
        assert_eq!(normalize_tag("v0.1.2"), "v0.1.2");
    }

    #[test]
    fn parse_checksum_line_finds_asset() {
        let data = "abc123  skytab-v0.1.2-aarch64-apple-darwin.tar.gz\n";
        let checksum = parse_checksum_line(data, "skytab-v0.1.2-aarch64-apple-darwin.tar.gz")
            .expect("checksum should parse");
        assert_eq!(checksum, "abc123");
    }

    #[test]
    fn parse_checksum_line_errors_when_missing() {
        let err = parse_checksum_line("abc123  other.tar.gz", "missing.tar.gz")
            .expect_err("missing asset should error");
        match err {
            SkyTabError::InvalidArgument(message) => {
                assert!(message.contains("checksum not found"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn is_update_available_uses_semver_for_latest() {
        assert!(is_update_available("v0.1.0", "v0.1.1", false));
        assert!(!is_update_available("v0.1.1", "v0.1.0", false));
        assert!(!is_update_available("v0.1.1", "v0.1.1", false));
    }

    #[test]
    fn explicit_version_can_downgrade() {
        assert!(is_update_available("v0.1.1", "v0.1.0", true));
    }
}
