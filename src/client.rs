use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};
use tracing::{debug, info, warn};

use crate::cache::TokenCache;
use crate::config::Config;
use crate::error::{Result, SkyTabError};

#[derive(Debug, Clone)]
pub struct SkyTabClient {
    pub base_url: String,
    username: Option<String>,
    password: Option<String>,
    http: reqwest::Client,
    token_cache: TokenCache,
    refresh_lock: Arc<Mutex<()>>,
}

impl SkyTabClient {
    pub fn new(config: Config) -> Self {
        Self::from_parts(
            config.base_url,
            Some(config.username),
            Some(config.password),
            TokenCache::new(),
        )
    }

    pub fn new_lazy(base_url: String) -> Self {
        Self::from_parts(base_url, None, None, TokenCache::new())
    }

    fn from_parts(
        base_url: String,
        username: Option<String>,
        password: Option<String>,
        token_cache: TokenCache,
    ) -> Self {
        Self {
            base_url,
            username,
            password,
            http: reqwest::Client::new(),
            token_cache,
            refresh_lock: Arc::new(Mutex::new(())),
        }
    }

    pub async fn token(&self, force_refresh: bool) -> Result<String> {
        if !force_refresh {
            if let Some(token) = self.token_cache.load_valid_token().await? {
                debug!(cache = "hit", "using cached auth token");
                return Ok(token);
            }
            debug!(cache = "miss", "no valid cached auth token");
        } else {
            info!("forcing auth token refresh");
        }

        let _refresh_guard = self.refresh_lock.lock().await;

        if !force_refresh {
            if let Some(token) = self.token_cache.load_valid_token().await? {
                debug!(cache = "hit_after_lock", "using cached auth token");
                return Ok(token);
            }
        }

        self.refresh_and_cache_token().await
    }

    pub async fn authenticate(&self) -> Result<String> {
        let (username, password) = self.resolve_auth_credentials().await?;
        let url = self.url("/api/v1/auth/authenticate");
        let started = Instant::now();
        debug!(
            endpoint = "/api/v1/auth/authenticate",
            "sending auth request"
        );
        let response = self
            .http
            .post(url)
            .header("content-type", "application/json")
            .json(&json!({
                "email": username,
                "password": password,
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            warn!(
                endpoint = "/api/v1/auth/authenticate",
                status,
                elapsed_ms = started.elapsed().as_millis() as u64,
                body_len = body.len(),
                "auth request failed"
            );
            return Err(SkyTabError::Api { status, body });
        }

        let json: Value = response.json().await?;
        let token = json
            .get("token")
            .and_then(Value::as_str)
            .ok_or_else(|| SkyTabError::InvalidArgument("missing token in auth response".into()))?;
        info!(
            endpoint = "/api/v1/auth/authenticate",
            elapsed_ms = started.elapsed().as_millis() as u64,
            "auth request succeeded"
        );
        Ok(token.to_string())
    }

    async fn resolve_auth_credentials(&self) -> Result<(String, String)> {
        if let (Some(username), Some(password)) = (&self.username, &self.password) {
            return Ok((username.clone(), password.clone()));
        }

        let config = Config::from_sources(Some(self.base_url.clone()))
            .await
            .map_err(map_auth_resolution_error)?;
        Ok((config.username, config.password))
    }

    async fn refresh_and_cache_token(&self) -> Result<String> {
        let token = self.authenticate().await?;
        self.token_cache.save_token(&token).await?;
        debug!("saved auth token to cache");
        Ok(token)
    }

    async fn refresh_token_after_unauthorized(&self, rejected_token: &str) -> Result<String> {
        let _refresh_guard = self.refresh_lock.lock().await;

        if let Some(token) = self.token_cache.load_valid_token().await? {
            if token != rejected_token {
                debug!(cache = "hit_after_401", "using refreshed cached auth token");
                return Ok(token);
            }
        }

        self.refresh_and_cache_token().await
    }

    pub async fn request_authed_json<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        query: &[(String, String)],
        body: Option<Value>,
    ) -> Result<T> {
        let token = self.token(false).await?;
        match self
            .request_json::<T>(method.clone(), path, query, body.clone(), Some(&token))
            .await
        {
            Ok(value) => Ok(value),
            Err(SkyTabError::Api { status: 401, .. }) => {
                info!(method = %method, path, "received 401, refreshing token and retrying once");
                let fresh = self.refresh_token_after_unauthorized(&token).await?;
                self.request_json(method, path, query, body, Some(&fresh))
                    .await
            }
            Err(err) => Err(err),
        }
    }

    pub async fn request_json<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        query: &[(String, String)],
        body: Option<Value>,
        access_token: Option<&str>,
    ) -> Result<T> {
        let max_retries = 5;
        let method_name = method.as_str().to_string();
        let query_count = query.len();
        let body_bytes = body
            .as_ref()
            .map(|payload| payload.to_string().len())
            .unwrap_or(0);

        for attempt in 0..=max_retries {
            let started = Instant::now();
            let mut req = self.http.request(method.clone(), self.url(path));
            if !query.is_empty() {
                req = req.query(query);
            }
            if let Some(token) = access_token {
                req = req.header("x-access-token", token);
            }
            if let Some(ref payload) = body {
                req = req.header("content-type", "application/json").json(payload);
            }

            debug!(
                method = %method_name,
                path,
                attempt,
                query_count,
                body_bytes,
                "sending api request"
            );

            let response = req.send().await?;
            let status = response.status();
            let elapsed_ms = started.elapsed().as_millis() as u64;

            if status.is_success() {
                info!(
                    method = %method_name,
                    path,
                    status = status.as_u16(),
                    attempt,
                    elapsed_ms,
                    "api request succeeded"
                );
                return Ok(response.json::<T>().await?);
            }

            let body_text = response.text().await.unwrap_or_default();
            let status_code = status.as_u16();
            let retryable = status_code == 429 || (500..=599).contains(&status_code);

            if retryable && attempt < max_retries {
                let delay_ms = (1u64 << attempt) * 500;
                warn!(
                    method = %method_name,
                    path,
                    status = status_code,
                    attempt,
                    elapsed_ms,
                    backoff_ms = delay_ms,
                    "api request retry scheduled"
                );
                sleep(Duration::from_millis(delay_ms)).await;
                continue;
            }

            warn!(
                method = %method_name,
                path,
                status = status_code,
                attempt,
                elapsed_ms,
                body_len = body_text.len(),
                "api request failed"
            );

            return Err(SkyTabError::Api {
                status: status_code,
                body: body_text,
            });
        }

        Err(SkyTabError::InvalidArgument(
            "request retry loop exhausted".into(),
        ))
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }
}

fn map_auth_resolution_error(err: SkyTabError) -> SkyTabError {
    match err {
        SkyTabError::MissingCredentials => SkyTabError::MissingCredentialsForAuthRefresh,
        SkyTabError::PartialEnvCredentials => SkyTabError::PartialEnvCredentialsForAuthRefresh,
        SkyTabError::CredentialStore(message) => {
            SkyTabError::CredentialStoreForAuthRefresh(message)
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::time::{SystemTime, UNIX_EPOCH};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::Barrier;

    #[test]
    fn auth_resolution_errors_are_contextualized_for_refresh() {
        assert!(matches!(
            map_auth_resolution_error(SkyTabError::MissingCredentials),
            SkyTabError::MissingCredentialsForAuthRefresh
        ));
        assert!(matches!(
            map_auth_resolution_error(SkyTabError::PartialEnvCredentials),
            SkyTabError::PartialEnvCredentialsForAuthRefresh
        ));
        assert!(matches!(
            map_auth_resolution_error(SkyTabError::CredentialStore("locked".to_string())),
            SkyTabError::CredentialStoreForAuthRefresh(message) if message == "locked"
        ));
    }

    #[tokio::test]
    async fn concurrent_cache_miss_triggers_single_authentication() {
        let (base_url, auth_hits, server_handle) = spawn_auth_server("fresh-token").await;
        let cache_path = unique_cache_path("single-auth");
        let token_cache = TokenCache::with_path(cache_path.clone(), 24);
        let client = Arc::new(SkyTabClient::from_parts(
            base_url,
            Some("alice@example.com".to_string()),
            Some("top-secret".to_string()),
            token_cache,
        ));
        let barrier = Arc::new(Barrier::new(8));
        let mut handles = Vec::new();

        for _ in 0..8 {
            let client = Arc::clone(&client);
            let barrier = Arc::clone(&barrier);
            handles.push(tokio::spawn(async move {
                barrier.wait().await;
                client.token(false).await
            }));
        }

        for handle in handles {
            let token = handle
                .await
                .expect("token task should join")
                .expect("token should resolve");
            assert_eq!(token, "fresh-token");
        }

        assert_eq!(auth_hits.load(Ordering::SeqCst), 1);

        server_handle.abort();
        cleanup_cache_path(&cache_path).await;
    }

    #[tokio::test]
    async fn unauthorized_retry_reuses_newer_cached_token() {
        let cache_path = unique_cache_path("reuse-newer-token");
        let token_cache = TokenCache::with_path(cache_path.clone(), 24);
        token_cache
            .save_token("new-token")
            .await
            .expect("token cache should save");

        let client =
            SkyTabClient::from_parts("http://127.0.0.1:9".to_string(), None, None, token_cache);

        let token = client
            .refresh_token_after_unauthorized("old-token")
            .await
            .expect("newer cached token should be reused");
        assert_eq!(token, "new-token");

        cleanup_cache_path(&cache_path).await;
    }

    async fn spawn_auth_server(
        token: &str,
    ) -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let addr = listener.local_addr().expect("listener should expose addr");
        let auth_hits = Arc::new(AtomicUsize::new(0));
        let auth_hits_ref = Arc::clone(&auth_hits);
        let token = token.to_string();

        let handle = tokio::spawn(async move {
            loop {
                let (mut stream, _) = match listener.accept().await {
                    Ok(pair) => pair,
                    Err(_) => break,
                };
                auth_hits_ref.fetch_add(1, Ordering::SeqCst);

                let mut buffer = [0_u8; 4096];
                let _ = stream.read(&mut buffer).await;

                let body = format!(r#"{{"token":"{}"}}"#, token);
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );

                let _ = stream.write_all(response.as_bytes()).await;
            }
        });

        (format!("http://{}", addr), auth_hits, handle)
    }

    fn unique_cache_path(test_name: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        let mut path = std::env::temp_dir();
        path.push("skytab-cli-tests");
        path.push(format!("{}-{}-{}", test_name, std::process::id(), now));
        path.push("token.json");
        path
    }

    async fn cleanup_cache_path(path: &PathBuf) {
        let _ = tokio::fs::remove_file(path).await;
        if let Some(parent) = path.parent() {
            let _ = tokio::fs::remove_dir(parent).await;
        }
    }
}
