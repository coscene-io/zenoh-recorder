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

// Filesystem backend implementation

use super::backend::StorageBackend;
use crate::config::FilesystemConfig;
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info, warn};

/// Filesystem backend for writing MCAP files to local disk
pub struct FilesystemBackend {
    base_path: PathBuf,
    file_format: String,
}

impl FilesystemBackend {
    pub fn new(config: FilesystemConfig) -> Result<Self> {
        let base_path = PathBuf::from(&config.base_path);
        
        info!(
            "Initializing filesystem backend at: {}",
            base_path.display()
        );
        
        Ok(Self {
            base_path,
            file_format: config.file_format,
        })
    }
    
    /// Ensure base directory exists
    async fn ensure_base_directory(&self) -> Result<()> {
        if !self.base_path.exists() {
            info!("Creating base directory: {}", self.base_path.display());
            fs::create_dir_all(&self.base_path)
                .await
                .context("Failed to create base directory")?;
        } else {
            info!(
                "Base directory already exists: {}",
                self.base_path.display()
            );
        }
        Ok(())
    }
    
    /// Get the file path for a given entry and timestamp
    fn get_file_path(&self, entry_name: &str, timestamp_us: u64) -> PathBuf {
        // Create a directory per entry
        let entry_dir = self.base_path.join(entry_name);
        
        // Create filename with timestamp
        let filename = format!("{}.{}", timestamp_us, self.file_format);
        entry_dir.join(filename)
    }
    
    /// Get metadata file path for storing labels
    fn get_metadata_path(&self, entry_name: &str, timestamp_us: u64) -> PathBuf {
        let entry_dir = self.base_path.join(entry_name);
        let filename = format!("{}.meta.json", timestamp_us);
        entry_dir.join(filename)
    }
    
    /// Ensure entry directory exists
    async fn ensure_entry_directory(&self, entry_name: &str) -> Result<()> {
        let entry_dir = self.base_path.join(entry_name);
        if !entry_dir.exists() {
            debug!("Creating entry directory: {}", entry_dir.display());
            fs::create_dir_all(&entry_dir)
                .await
                .context("Failed to create entry directory")?;
        }
        Ok(())
    }
}

#[async_trait]
impl StorageBackend for FilesystemBackend {
    async fn initialize(&self) -> Result<()> {
        self.ensure_base_directory().await
    }
    
    async fn write_record(
        &self,
        entry_name: &str,
        timestamp_us: u64,
        data: Vec<u8>,
        labels: HashMap<String, String>,
    ) -> Result<()> {
        // Ensure entry directory exists
        self.ensure_entry_directory(entry_name).await?;
        
        // Get file paths
        let file_path = self.get_file_path(entry_name, timestamp_us);
        let metadata_path = self.get_metadata_path(entry_name, timestamp_us);
        
        // Write data file
        debug!(
            "Writing {} bytes to {}",
            data.len(),
            file_path.display()
        );
        
        let mut file = fs::File::create(&file_path)
            .await
            .context(format!("Failed to create file: {}", file_path.display()))?;
        
        file.write_all(&data)
            .await
            .context("Failed to write data")?;
        
        file.flush().await.context("Failed to flush data")?;
        
        // Write metadata file with labels
        if !labels.is_empty() {
            debug!(
                "Writing metadata to {}",
                metadata_path.display()
            );
            
            let metadata_json = serde_json::to_string_pretty(&labels)
                .context("Failed to serialize metadata")?;
            
            let mut meta_file = fs::File::create(&metadata_path)
                .await
                .context(format!(
                    "Failed to create metadata file: {}",
                    metadata_path.display()
                ))?;
            
            meta_file
                .write_all(metadata_json.as_bytes())
                .await
                .context("Failed to write metadata")?;
            
            meta_file.flush().await.context("Failed to flush metadata")?;
        }
        
        info!(
            "Successfully wrote {} bytes to entry '{}' at timestamp {}",
            data.len(),
            entry_name,
            timestamp_us
        );
        
        Ok(())
    }
    
    async fn health_check(&self) -> Result<bool> {
        // Check if base directory is accessible and writable
        match fs::metadata(&self.base_path).await {
            Ok(metadata) if metadata.is_dir() => {
                // Try to create a temporary test file to verify write permissions
                let test_file = self.base_path.join(".health_check_test");
                match fs::File::create(&test_file).await {
                    Ok(mut f) => {
                        // Write a test byte
                        if let Err(e) = f.write_all(b"test").await {
                            warn!("Health check failed - cannot write: {}", e);
                            return Ok(false);
                        }
                        // Clean up test file
                        let _ = fs::remove_file(&test_file).await;
                        Ok(true)
                    }
                    Err(e) => {
                        warn!("Health check failed - cannot create file: {}", e);
                        Ok(false)
                    }
                }
            }
            Ok(_) => {
                warn!(
                    "Health check failed - base path is not a directory: {}",
                    self.base_path.display()
                );
                Ok(false)
            }
            Err(e) => {
                warn!(
                    "Health check failed - cannot access base path {}: {}",
                    self.base_path.display(),
                    e
                );
                Ok(false)
            }
        }
    }
    
    fn backend_type(&self) -> &str {
        "filesystem"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    fn create_test_backend() -> (FilesystemBackend, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = FilesystemConfig {
            base_path: temp_dir.path().to_string_lossy().to_string(),
            file_format: "mcap".to_string(),
        };
        let backend = FilesystemBackend::new(config).unwrap();
        (backend, temp_dir)
    }
    
    #[tokio::test]
    async fn test_initialize() {
        let (backend, _temp_dir) = create_test_backend();
        let result = backend.initialize().await;
        assert!(result.is_ok());
        assert!(backend.base_path.exists());
    }
    
    #[tokio::test]
    async fn test_write_record() {
        let (backend, _temp_dir) = create_test_backend();
        backend.initialize().await.unwrap();
        
        let entry_name = "test_entry";
        let timestamp_us = 1234567890;
        let data = b"test data".to_vec();
        let mut labels = HashMap::new();
        labels.insert("recording_id".to_string(), "test-123".to_string());
        labels.insert("topic".to_string(), "/test/topic".to_string());
        
        let result = backend
            .write_record(entry_name, timestamp_us, data.clone(), labels.clone())
            .await;
        
        assert!(result.is_ok());
        
        // Verify data file exists
        let file_path = backend.get_file_path(entry_name, timestamp_us);
        assert!(file_path.exists());
        
        // Verify data content
        let written_data = std::fs::read(&file_path).unwrap();
        assert_eq!(written_data, data);
        
        // Verify metadata file exists
        let metadata_path = backend.get_metadata_path(entry_name, timestamp_us);
        assert!(metadata_path.exists());
        
        // Verify metadata content
        let metadata_content = std::fs::read_to_string(&metadata_path).unwrap();
        let parsed_labels: HashMap<String, String> =
            serde_json::from_str(&metadata_content).unwrap();
        assert_eq!(parsed_labels, labels);
    }
    
    #[tokio::test]
    async fn test_health_check() {
        let (backend, _temp_dir) = create_test_backend();
        backend.initialize().await.unwrap();
        
        let result = backend.health_check().await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
    
    #[tokio::test]
    async fn test_multiple_entries() {
        let (backend, _temp_dir) = create_test_backend();
        backend.initialize().await.unwrap();
        
        // Write to multiple entries
        for i in 0..3 {
            let entry_name = format!("entry_{}", i);
            let timestamp_us = 1000000 + i;
            let data = format!("data_{}", i).into_bytes();
            let labels = HashMap::new();
            
            let result = backend
                .write_record(&entry_name, timestamp_us, data, labels)
                .await;
            assert!(result.is_ok());
        }
        
        // Verify all entry directories exist
        for i in 0..3 {
            let entry_dir = backend.base_path.join(format!("entry_{}", i));
            assert!(entry_dir.exists());
        }
    }
}

