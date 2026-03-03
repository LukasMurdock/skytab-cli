use std::{env, path::PathBuf};

use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::error::{Result, SkyTabError};

pub const DEFAULT_BASE_URL: &str = "https://lighthouse-api.harbortouch.com";

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

impl Config {
    pub async fn from_sources(base_url: Option<String>) -> Result<Self> {
        let from_file = load_file_config().await?;

        let username = env::var("SKYTAB_USERNAME")
            .ok()
            .or_else(|| from_file.as_ref().and_then(|cfg| cfg.username.clone()));
        let password = env::var("SKYTAB_PASSWORD")
            .ok()
            .or_else(|| from_file.as_ref().and_then(|cfg| cfg.password.clone()));
        let base_url = base_url
            .or_else(|| env::var("SKYTAB_BASE_URL").ok())
            .or_else(|| from_file.as_ref().and_then(|cfg| cfg.base_url.clone()))
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

        let username = username.ok_or(SkyTabError::MissingCredentials)?;
        let password = password.ok_or(SkyTabError::MissingCredentials)?;

        Ok(Self {
            base_url,
            username,
            password,
        })
    }
}

pub async fn save_credentials(
    username: String,
    password: String,
    base_url: Option<String>,
) -> Result<PathBuf> {
    let path = config_file_path();
    let mut existing = load_file_config().await?.unwrap_or_default();
    existing.username = Some(username);
    existing.password = Some(password);
    if base_url.is_some() {
        existing.base_url = base_url;
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }

    let content = toml::to_string_pretty(&existing)?;
    fs::write(&path, content).await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(path)
}

pub async fn get_default_location_id() -> Result<Option<i64>> {
    let from_file = load_file_config().await?;
    Ok(env::var("SKYTAB_DEFAULT_LOCATION_ID")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .or_else(|| from_file.and_then(|cfg| cfg.default_location_id)))
}

pub async fn save_default_location_id(location_id: i64) -> Result<PathBuf> {
    let path = config_file_path();
    let mut existing = load_file_config().await?.unwrap_or_default();
    existing.default_location_id = Some(location_id);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }

    let content = toml::to_string_pretty(&existing)?;
    fs::write(&path, content).await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(path)
}

pub async fn clear_default_location_id() -> Result<PathBuf> {
    let path = config_file_path();
    let mut existing = load_file_config().await?.unwrap_or_default();
    existing.default_location_id = None;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }

    let content = toml::to_string_pretty(&existing)?;
    fs::write(&path, content).await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(path)
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

fn config_file_path() -> PathBuf {
    let mut dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".config"));
    dir.push("skytab");
    dir.push("config.toml");
    dir
}

async fn load_file_config() -> Result<Option<FileConfig>> {
    let path = config_file_path();
    let content = match read_config_content(path).await {
        Ok(v) => v,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };

    let parsed = toml::from_str::<FileConfig>(&content)?;
    Ok(Some(parsed))
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
