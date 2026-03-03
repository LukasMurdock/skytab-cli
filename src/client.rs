use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::time::Instant;
use tokio::time::{Duration, sleep};
use tracing::{debug, info, warn};

use crate::cache::TokenCache;
use crate::config::Config;
use crate::error::{Result, SkyTabError};

#[derive(Debug, Clone)]
pub struct SkyTabClient {
    pub base_url: String,
    pub username: String,
    pub password: String,
    http: reqwest::Client,
    token_cache: TokenCache,
}

impl SkyTabClient {
    pub fn new(config: Config) -> Self {
        Self {
            base_url: config.base_url,
            username: config.username,
            password: config.password,
            http: reqwest::Client::new(),
            token_cache: TokenCache::new(),
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

        let token = self.authenticate().await?;
        self.token_cache.save_token(&token).await?;
        debug!("saved auth token to cache");
        Ok(token)
    }

    pub async fn authenticate(&self) -> Result<String> {
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
                "email": self.username,
                "password": self.password,
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
                let fresh = self.token(true).await?;
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
