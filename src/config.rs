use std::{
    env,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::warn;

use crate::error::{Result, SkyTabError};

pub const DEFAULT_BASE_URL: &str = "https://lighthouse-api.harbortouch.com";
const KEYRING_SERVICE_NAME: &str = "skytab-cli";

#[derive(Debug, Clone)]
pub struct Config {
    pub base_url: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct FileConfig {
    base_url: Option<String>,
    username: Option<String>,
    password: Option<String>,
    default_location_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CredentialStorageDiagnostics {
    pub mode: String,
    pub keyring_supported: bool,
    pub keyring_accessible: bool,
    pub keyring_password_present: bool,
    pub config_password_present: bool,
    pub username_present: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CredentialStoreMode {
    Auto,
    Keyring,
    Config,
}

impl CredentialStoreMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Keyring => "keyring",
            Self::Config => "config",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PasswordStorage {
    Keyring,
    ConfigFile,
}

impl Config {
    pub async fn from_sources(base_url: Option<String>) -> Result<Self> {
        let from_file = load_file_config().await?;
        let resolved_base_url = resolve_base_url(base_url, from_file.as_ref());

        let env_username = env::var("SKYTAB_USERNAME").ok();
        let env_password = env::var("SKYTAB_PASSWORD").ok();

        match (env_username, env_password) {
            (Some(username), Some(password)) => {
                return Ok(Self {
                    base_url: resolved_base_url,
                    username,
                    password,
                });
            }
            (Some(_), None) | (None, Some(_)) => {
                return Err(SkyTabError::PartialEnvCredentials);
            }
            (None, None) => {}
        }

        let username = from_file
            .as_ref()
            .and_then(|cfg| cfg.username.clone())
            .ok_or(SkyTabError::MissingCredentials)?;

        let mode = credential_store_mode();
        let password =
            resolve_persisted_password(&resolved_base_url, &username, from_file.as_ref(), mode)
                .await?;

        Ok(Self {
            base_url: resolved_base_url,
            username,
            password,
        })
    }
}

pub async fn resolve_base_url_from_sources(base_url_override: Option<String>) -> Result<String> {
    let from_file = load_file_config().await?;
    Ok(resolve_base_url(base_url_override, from_file.as_ref()))
}

pub async fn save_credentials(
    username: String,
    password: String,
    base_url: Option<String>,
) -> Result<PathBuf> {
    let mut existing = load_file_config().await?.unwrap_or_default();
    existing.username = Some(username.clone());

    if let Some(base_url) = base_url {
        existing.base_url = Some(base_url);
    }

    let resolved_base_url = existing
        .base_url
        .clone()
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

    let mode = credential_store_mode();
    let storage = store_password(&resolved_base_url, &username, &password, mode)?;

    match storage {
        PasswordStorage::Keyring => {
            existing.password = None;
        }
        PasswordStorage::ConfigFile => {
            existing.password = Some(password);
        }
    }

    save_file_config(&existing).await
}

pub async fn get_default_location_id() -> Result<Option<i64>> {
    let from_file = load_file_config().await?;
    Ok(env::var("SKYTAB_DEFAULT_LOCATION_ID")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .or_else(|| from_file.and_then(|cfg| cfg.default_location_id)))
}

pub async fn save_default_location_id(location_id: i64) -> Result<PathBuf> {
    let mut existing = load_file_config().await?.unwrap_or_default();
    existing.default_location_id = Some(location_id);
    save_file_config(&existing).await
}

pub async fn clear_default_location_id() -> Result<PathBuf> {
    let mut existing = load_file_config().await?.unwrap_or_default();
    existing.default_location_id = None;
    save_file_config(&existing).await
}

pub async fn credential_storage_diagnostics(
    base_url_override: Option<String>,
) -> Result<CredentialStorageDiagnostics> {
    let from_file = load_file_config().await?;
    let resolved_base_url = resolve_base_url(base_url_override, from_file.as_ref());
    let mode = credential_store_mode();

    let username = env::var("SKYTAB_USERNAME")
        .ok()
        .or_else(|| from_file.as_ref().and_then(|cfg| cfg.username.clone()));

    let keyring_supported = keyring_backend_supported();
    let (keyring_accessible, keyring_password_present) = if !keyring_supported {
        (false, false)
    } else if let Some(ref username) = username {
        match load_password_from_keyring(&resolved_base_url, username) {
            Ok(Some(_)) => (true, true),
            Ok(None) => (true, false),
            Err(_) => (false, false),
        }
    } else {
        (true, false)
    };

    let config_password_present = from_file
        .as_ref()
        .and_then(|cfg| cfg.password.as_ref())
        .map(|password| !password.is_empty())
        .unwrap_or(false);

    Ok(CredentialStorageDiagnostics {
        mode: mode.as_str().to_string(),
        keyring_supported,
        keyring_accessible,
        keyring_password_present,
        config_password_present,
        username_present: username.is_some(),
    })
}

pub fn current_config_file_path() -> PathBuf {
    config_file_path()
}

pub fn legacy_config_file_path() -> PathBuf {
    let mut legacy = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".config"));
    legacy.push("skytab-cli");
    legacy.push("config.toml");
    legacy
}

fn resolve_base_url(base_url_override: Option<String>, from_file: Option<&FileConfig>) -> String {
    base_url_override
        .or_else(|| env::var("SKYTAB_BASE_URL").ok())
        .or_else(|| from_file.and_then(|cfg| cfg.base_url.clone()))
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
}

async fn resolve_persisted_password(
    base_url: &str,
    username: &str,
    from_file: Option<&FileConfig>,
    mode: CredentialStoreMode,
) -> Result<String> {
    if mode != CredentialStoreMode::Config {
        match load_password_from_keyring(base_url, username) {
            Ok(Some(password)) => return Ok(password),
            Ok(None) => {}
            Err(err) => {
                if mode == CredentialStoreMode::Keyring {
                    return Err(SkyTabError::CredentialStore(err));
                }
                warn!(
                    mode = %mode.as_str(),
                    error = %err,
                    "unable to read password from keyring; falling back to config file"
                );
            }
        }
    }

    if let Some(legacy_password) = from_file
        .and_then(|cfg| cfg.password.clone())
        .filter(|password| !password.is_empty())
    {
        if mode != CredentialStoreMode::Config {
            match save_password_to_keyring(base_url, username, &legacy_password) {
                Ok(()) => {
                    if let Some(file_config) = from_file.cloned() {
                        let mut scrubbed = file_config;
                        scrubbed.password = None;
                        if let Err(err) = save_file_config(&scrubbed).await {
                            warn!(error = %err, "unable to remove legacy plaintext password from config");
                        }
                    }
                }
                Err(err) => {
                    warn!(
                        error = %err,
                        "unable to migrate legacy plaintext password to keyring"
                    );
                }
            }
        }

        return Ok(legacy_password);
    }

    Err(SkyTabError::MissingCredentials)
}

fn store_password(
    base_url: &str,
    username: &str,
    password: &str,
    mode: CredentialStoreMode,
) -> Result<PasswordStorage> {
    match mode {
        CredentialStoreMode::Config => Ok(PasswordStorage::ConfigFile),
        CredentialStoreMode::Keyring => {
            save_password_to_keyring(base_url, username, password)
                .map_err(SkyTabError::CredentialStore)?;
            Ok(PasswordStorage::Keyring)
        }
        CredentialStoreMode::Auto => match save_password_to_keyring(base_url, username, password) {
            Ok(()) => Ok(PasswordStorage::Keyring),
            Err(err) => {
                warn!(
                    error = %err,
                    "unable to persist password in keyring; falling back to config file"
                );
                Ok(PasswordStorage::ConfigFile)
            }
        },
    }
}

fn credential_store_mode() -> CredentialStoreMode {
    let raw = env::var("SKYTAB_CREDENTIAL_STORE").ok();
    parse_credential_store_mode(raw.as_deref())
}

fn parse_credential_store_mode(value: Option<&str>) -> CredentialStoreMode {
    match value.unwrap_or("auto").trim().to_ascii_lowercase().as_str() {
        "keyring" => CredentialStoreMode::Keyring,
        "config" => CredentialStoreMode::Config,
        _ => CredentialStoreMode::Auto,
    }
}

fn keyring_account(base_url: &str, username: &str) -> String {
    format!("{}|{}", base_url.trim_end_matches('/'), username.trim())
}

fn keyring_backend_supported() -> bool {
    cfg!(any(
        target_os = "macos",
        all(target_os = "linux", not(target_env = "musl"))
    ))
}

#[cfg(any(
    target_os = "macos",
    all(target_os = "linux", not(target_env = "musl"))
))]
fn load_password_from_keyring(
    base_url: &str,
    username: &str,
) -> std::result::Result<Option<String>, String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE_NAME, &keyring_account(base_url, username))
        .map_err(|err| err.to_string())?;
    match entry.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(err.to_string()),
    }
}

#[cfg(not(any(
    target_os = "macos",
    all(target_os = "linux", not(target_env = "musl"))
)))]
fn load_password_from_keyring(
    _base_url: &str,
    _username: &str,
) -> std::result::Result<Option<String>, String> {
    Err("keyring backend is unavailable for this build target".to_string())
}

#[cfg(any(
    target_os = "macos",
    all(target_os = "linux", not(target_env = "musl"))
))]
fn save_password_to_keyring(
    base_url: &str,
    username: &str,
    password: &str,
) -> std::result::Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE_NAME, &keyring_account(base_url, username))
        .map_err(|err| err.to_string())?;
    entry.set_password(password).map_err(|err| err.to_string())
}

#[cfg(not(any(
    target_os = "macos",
    all(target_os = "linux", not(target_env = "musl"))
)))]
fn save_password_to_keyring(
    _base_url: &str,
    _username: &str,
    _password: &str,
) -> std::result::Result<(), String> {
    Err("keyring backend is unavailable for this build target".to_string())
}

fn config_file_path() -> PathBuf {
    let mut dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".config"));
    dir.push("skytab");
    dir.push("config.toml");
    dir
}

async fn load_file_config() -> Result<Option<FileConfig>> {
    let path = config_file_path();
    let content = match read_config_content(path).await {
        Ok(value) => value,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };

    let parsed = toml::from_str::<FileConfig>(&content)?;
    Ok(Some(parsed))
}

async fn save_file_config(config: &FileConfig) -> Result<PathBuf> {
    let path = config_file_path();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }

    let content = toml::to_string_pretty(config)?;
    fs::write(&path, content).await?;
    set_unix_private_permissions(&path)?;

    Ok(path)
}

fn set_unix_private_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

async fn read_config_content(path: PathBuf) -> std::io::Result<String> {
    match fs::read_to_string(&path).await {
        Ok(content) => Ok(content),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let mut legacy = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".config"));
            legacy.push("skytab-cli");
            legacy.push("config.toml");
            fs::read_to_string(legacy).await
        }
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_credential_store_mode_defaults_to_auto() {
        assert_eq!(parse_credential_store_mode(None), CredentialStoreMode::Auto);
        assert_eq!(
            parse_credential_store_mode(Some("")),
            CredentialStoreMode::Auto
        );
        assert_eq!(
            parse_credential_store_mode(Some("unexpected-value")),
            CredentialStoreMode::Auto
        );
    }

    #[test]
    fn parse_credential_store_mode_recognizes_keyring_and_config() {
        assert_eq!(
            parse_credential_store_mode(Some("keyring")),
            CredentialStoreMode::Keyring
        );
        assert_eq!(
            parse_credential_store_mode(Some("CONFIG")),
            CredentialStoreMode::Config
        );
    }

    #[test]
    fn keyring_account_uses_base_url_and_username() {
        assert_eq!(
            keyring_account("https://example.com/", "alice@example.com"),
            "https://example.com|alice@example.com"
        );
    }
}
