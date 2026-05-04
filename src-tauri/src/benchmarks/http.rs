//! HTTP fetcher trait. The benchmark providers depend on this trait
//! rather than `reqwest::Client` directly so tests can swap in a
//! `MockHttpFetcher` and exercise parser logic against canned response
//! bodies without network.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;

use super::error::BenchmarkError;

#[async_trait]
pub trait HttpFetcher: Send + Sync {
    async fn fetch(&self, url: &str) -> Result<String, BenchmarkError>;
}

/// Production fetcher backed by reqwest with rustls-tls.
pub struct ReqwestFetcher {
    client: Client,
}

impl Default for ReqwestFetcher {
    fn default() -> Self {
        // 15s total timeout, 5s connect timeout. Without these, a slow
        // upstream (Aave/Lido via DefiLlama have been observed hanging
        // with no error) blocks Promise.allSettled forever — turning a
        // single bad benchmark into a stuck pipeline. With timeouts the
        // hang surfaces as a real Http error and the rest of the
        // pipeline progresses.
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { client }
    }
}

#[async_trait]
impl HttpFetcher for ReqwestFetcher {
    async fn fetch(&self, url: &str) -> Result<String, BenchmarkError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| BenchmarkError::Http(e.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            return Err(BenchmarkError::Status(status.as_u16()));
        }
        let body = response
            .text()
            .await
            .map_err(|e| BenchmarkError::Http(e.to_string()))?;
        Ok(body)
    }
}

/// Test fetcher seeded with a URL → response body map.
pub struct MockHttpFetcher {
    responses: Mutex<HashMap<String, String>>,
}

impl Default for MockHttpFetcher {
    fn default() -> Self {
        Self {
            responses: Mutex::new(HashMap::new()),
        }
    }
}

impl MockHttpFetcher {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn insert(&self, url: impl Into<String>, body: impl Into<String>) {
        self.responses
            .lock()
            .unwrap()
            .insert(url.into(), body.into());
    }
}

#[async_trait]
impl HttpFetcher for MockHttpFetcher {
    async fn fetch(&self, url: &str) -> Result<String, BenchmarkError> {
        self.responses
            .lock()
            .unwrap()
            .get(url)
            .cloned()
            .ok_or_else(|| BenchmarkError::Http(format!("no mock for url {url}")))
    }
}
