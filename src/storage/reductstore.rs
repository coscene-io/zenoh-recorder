// Copyright 2025 coScene
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// ReductStore backend implementation

use super::backend::StorageBackend;
use crate::config::ReductStoreConfig;
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn};

/// ReductStore client for uploading data
pub struct ReductStoreBackend {
    client: Client,
    base_url: String,
    bucket_name: String,
    max_retries: u32,
}

impl ReductStoreBackend {
    pub fn new(config: ReductStoreConfig) -> Result<Self> {
        let mut client_builder = reqwest::ClientBuilder::new()
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Duration::from_secs(60))
            .timeout(Duration::from_secs(config.timeout_seconds));

        // Add API token if provided
        if let Some(token) = &config.api_token {
            let mut headers = reqwest::header::HeaderMap::new();
            let auth_value = format!("Bearer {}", token);
            headers.insert(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&auth_value).context("Invalid API token")?,
            );
            client_builder = client_builder.default_headers(headers);
        }

        let client = client_builder
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self {
            client,
            base_url: config.url,
            bucket_name: config.bucket_name,
            max_retries: config.max_retries,
        })
    }

    /// Create bucket if it doesn't exist
    async fn ensure_bucket(&self) -> Result<()> {
        let url = format!("{}/api/v1/b/{}", self.base_url, self.bucket_name);

        match self.client.head(&url).send().await {
            Ok(response) if response.status().is_success() => {
                info!("Bucket '{}' already exists", self.bucket_name);
                Ok(())
            }
            _ => {
                info!("Creating bucket '{}'", self.bucket_name);
                let create_url = format!("{}/api/v1/b/{}", self.base_url, self.bucket_name);
                let response = self
                    .client
                    .post(&create_url)
                    .send()
                    .await
                    .context("Failed to create bucket")?;

                if response.status().is_success() || response.status().as_u16() == 409 {
                    info!("Bucket '{}' created successfully", self.bucket_name);
                    Ok(())
                } else {
                    let status = response.status();
                    let error_text = response.text().await.unwrap_or_default();
                    bail!("Failed to create bucket: {} - {}", status, error_text)
                }
            }
        }
    }
}

#[async_trait]
impl StorageBackend for ReductStoreBackend {
    async fn initialize(&self) -> Result<()> {
        self.ensure_bucket().await
    }

    async fn write_record(
        &self,
        entry_name: &str,
        timestamp_us: u64,
        data: Vec<u8>,
        labels: HashMap<String, String>,
    ) -> Result<()> {
        let url = format!(
            "{}/api/v1/b/{}/{}?ts={}",
            self.base_url, self.bucket_name, entry_name, timestamp_us
        );

        let mut request = self
            .client
            .post(&url)
            .header("Content-Type", "application/octet-stream");

        // Add labels as headers
        for (key, value) in labels {
            request = request.header(format!("x-reduct-label-{}", key), value);
        }

        let response = request
            .body(data)
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            bail!(
                "ReductStore write failed with status {}: {}",
                status,
                error_text
            );
        }

        Ok(())
    }

    async fn write_with_retry(
        &self,
        entry_name: &str,
        timestamp_us: u64,
        data: Vec<u8>,
        labels: HashMap<String, String>,
        max_retries: u32,
    ) -> Result<()> {
        // Use the configured max_retries or override
        let retries = if max_retries > 0 {
            max_retries
        } else {
            self.max_retries
        };

        // Call the default trait implementation
        let mut attempt = 0;
        let mut delay = Duration::from_millis(100);

        loop {
            match self
                .write_record(entry_name, timestamp_us, data.clone(), labels.clone())
                .await
            {
                Ok(_) => {
                    if attempt > 0 {
                        info!(
                            "Successfully uploaded to entry '{}' after {} retries",
                            entry_name, attempt
                        );
                    }
                    return Ok(());
                }
                Err(e) if attempt < retries => {
                    warn!(
                        "Upload to entry '{}' failed (attempt {}/{}): {}. Retrying in {:?}",
                        entry_name,
                        attempt + 1,
                        retries,
                        e,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                    delay *= 2; // Exponential backoff
                    delay = delay.min(Duration::from_secs(30)); // Cap at 30 seconds
                    attempt += 1;
                }
                Err(e) => {
                    tracing::error!(
                        "Upload to entry '{}' failed after {} attempts: {}",
                        entry_name,
                        retries,
                        e
                    );
                    return Err(e);
                }
            }
        }
    }

    async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/api/v1/info", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => Ok(true),
            Ok(response) => {
                warn!("Health check failed with status: {}", response.status());
                Ok(false)
            }
            Err(e) => {
                warn!("Health check error: {}", e);
                Ok(false)
            }
        }
    }

    fn backend_type(&self) -> &str {
        "reductstore"
    }
}

/// Convert Zenoh topic to ReductStore entry name
pub fn topic_to_entry_name(topic: &str) -> String {
    topic
        .trim_start_matches('/')
        .replace('/', "_")
        .replace("**", "all")
}
