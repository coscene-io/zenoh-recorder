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

// Storage backend trait for write-only recording

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;

/// Generic storage backend trait for write-only recording
///
/// This trait defines the interface for storage backends that the recorder
/// can write data to. Implementations should focus on efficient writes.
///
/// Query operations are NOT part of this trait - users should query
/// backends directly using their specialized tools (ReductStore UI, Grafana, etc.)
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Initialize the backend (create bucket/database if needed)
    async fn initialize(&self) -> Result<()>;

    /// Write a single record with metadata
    ///
    /// # Arguments
    /// * `entry_name` - Entry/stream name for the data
    /// * `timestamp_us` - Timestamp in microseconds
    /// * `data` - Binary data to store
    /// * `labels` - Metadata labels/tags
    async fn write_record(
        &self,
        entry_name: &str,
        timestamp_us: u64,
        data: Vec<u8>,
        labels: HashMap<String, String>,
    ) -> Result<()>;

    /// Write with retry logic (optional, has default implementation)
    ///
    /// # Arguments
    /// * `entry_name` - Entry/stream name for the data
    /// * `timestamp_us` - Timestamp in microseconds
    /// * `data` - Binary data to store
    /// * `labels` - Metadata labels/tags
    /// * `max_retries` - Maximum number of retry attempts
    async fn write_with_retry(
        &self,
        entry_name: &str,
        timestamp_us: u64,
        data: Vec<u8>,
        labels: HashMap<String, String>,
        max_retries: u32,
    ) -> Result<()> {
        use tokio::time::{sleep, Duration};
        use tracing::{info, warn};

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
                    tracing::error!(
                        "Upload to entry '{}' failed after {} attempts: {}",
                        entry_name,
                        max_retries,
                        e
                    );
                    return Err(e);
                }
            }
        }
    }

    /// Health check
    async fn health_check(&self) -> Result<bool>;

    /// Get backend type identifier
    fn backend_type(&self) -> &str;
}
