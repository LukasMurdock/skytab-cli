use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;

use crate::error::Result;

#[derive(Debug, Clone)]
pub struct TokenCache {
    path: std::path::PathBuf,
    ttl_hours: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenRecord {
    token: String,
    expires_at: DateTime<Utc>,
}

impl TokenCache {
    pub fn new() -> Self {
        let mut dir = dirs::cache_dir().unwrap_or_else(|| std::path::PathBuf::from(".cache"));
        dir.push("skytab");
        let mut path = dir;
        path.push("token.json");
        Self {
            path,
            ttl_hours: 24,
        }
    }

    pub async fn load_valid_token(&self) -> Result<Option<String>> {
        let content = match read_cache_content(&self.path).await {
            Ok(v) => v,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err.into()),
        };

        let record: TokenRecord = serde_json::from_str(&content)?;
        if record.expires_at > Utc::now() {
            return Ok(Some(record.token));
        }

        Ok(None)
    }

    pub async fn save_token(&self, token: &str) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let record = TokenRecord {
            token: token.to_string(),
            expires_at: Utc::now() + Duration::hours(self.ttl_hours),
        };
        let serialized = serde_json::to_string_pretty(&record)?;
        fs::write(&self.path, serialized).await?;
        set_unix_private_permissions(&self.path)?;
        Ok(())
    }

    pub fn path(&self) -> std::path::PathBuf {
        self.path.clone()
    }

    pub fn legacy_path() -> std::path::PathBuf {
        let mut legacy = dirs::cache_dir().unwrap_or_else(|| std::path::PathBuf::from(".cache"));
        legacy.push("skytab-cli");
        legacy.push("token.json");
        legacy
    }

    #[cfg(test)]
    pub(crate) fn with_path(path: std::path::PathBuf, ttl_hours: i64) -> Self {
        Self { path, ttl_hours }
    }
}

async fn read_cache_content(path: &std::path::Path) -> std::io::Result<String> {
    match fs::read_to_string(path).await {
        Ok(content) => Ok(content),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let mut legacy =
                dirs::cache_dir().unwrap_or_else(|| std::path::PathBuf::from(".cache"));
            legacy.push("skytab-cli");
            legacy.push("token.json");
            fs::read_to_string(legacy).await
        }
        Err(err) => Err(err),
    }
}

fn set_unix_private_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}
