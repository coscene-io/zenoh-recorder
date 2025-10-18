use anyhow::{bail, Context, Result};
use reqwest::Client;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

/// ReductStore client for uploading data
pub struct ReductStoreClient {
    client: Client,
    base_url: String,
    bucket_name: String,
}

impl ReductStoreClient {
    pub fn new(base_url: String, bucket_name: String) -> Self {
        let client = reqwest::ClientBuilder::new()
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Duration::from_secs(60))
            .timeout(Duration::from_secs(300)) // 5 minute timeout for large uploads
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url,
            bucket_name,
        }
    }

    /// Write a record to ReductStore
    pub async fn write_record(
        &self,
        entry_name: &str,
        timestamp_us: u64,
        data: Vec<u8>,
        labels: HashMap<String, String>,
    ) -> Result<()> {
        let url = format!(
            "{}/api/v1/b/{}/{}",
            self.base_url, self.bucket_name, entry_name
        );

        let mut request = self
            .client
            .post(&url)
            .header("Content-Type", "application/octet-stream")
            .header("x-reduct-time", timestamp_us.to_string());

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

    /// Write a record with retry logic
    pub async fn write_record_with_retry(
        &self,
        entry_name: &str,
        timestamp_us: u64,
        data: Vec<u8>,
        labels: HashMap<String, String>,
        max_retries: u32,
    ) -> Result<()> {
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
                Err(e) if attempt < max_retries => {
                    warn!(
                        "Upload to entry '{}' failed (attempt {}/{}): {}. Retrying in {:?}",
                        entry_name,
                        attempt + 1,
                        max_retries,
                        e,
                        delay
                    );
                    sleep(delay).await;
                    delay *= 2; // Exponential backoff
                    delay = delay.min(Duration::from_secs(30)); // Cap at 30 seconds
                    attempt += 1;
                }
                Err(e) => {
                    error!(
                        "Upload to entry '{}' failed after {} attempts: {}",
                        entry_name, max_retries, e
                    );
                    return Err(e);
                }
            }
        }
    }

    /// Create bucket if it doesn't exist
    pub async fn ensure_bucket(&self) -> Result<()> {
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
                    bail!("Failed to create bucket: {}", response.status())
                }
            }
        }
    }
}

/// Convert Zenoh topic to ReductStore entry name
pub fn topic_to_entry_name(topic: &str) -> String {
    topic
        .trim_start_matches('/')
        .replace('/', "_")
        .replace("**", "all")
}
