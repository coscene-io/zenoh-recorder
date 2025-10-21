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

// Configuration types for zenoh-recorder

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Main configuration structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecorderConfig {
    pub zenoh: ZenohConfig,
    pub storage: StorageConfig,
    pub recorder: RecorderSettings,
    #[serde(default)]
    pub logging: LoggingConfig,
}

impl Default for RecorderConfig {
    fn default() -> Self {
        Self {
            zenoh: ZenohConfig::default(),
            storage: StorageConfig::default(),
            recorder: RecorderSettings::default(),
            logging: LoggingConfig::default(),
        }
    }
}

/// Zenoh configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ZenohConfig {
    #[serde(default = "default_mode")]
    pub mode: String,  // "peer", "client", or "router"
    
    #[serde(default)]
    pub connect: Option<ConnectConfig>,
    
    #[serde(default)]
    pub listen: Option<ListenConfig>,
}

impl Default for ZenohConfig {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            connect: Some(ConnectConfig {
                endpoints: vec!["tcp/localhost:7447".to_string()],
            }),
            listen: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnectConfig {
    pub endpoints: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListenConfig {
    pub endpoints: Vec<String>,
}

/// Storage configuration with backend selection
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageConfig {
    /// Backend type: "reductstore", "filesystem", "influxdb", "s3"
    pub backend: String,
    
    /// Backend-specific configuration
    #[serde(flatten)]
    pub backend_config: BackendConfig,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: "reductstore".to_string(),
            backend_config: BackendConfig::ReductStore {
                reductstore: ReductStoreConfig::default(),
            },
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BackendConfig {
    ReductStore {
        #[serde(rename = "reductstore")]
        reductstore: ReductStoreConfig,
    },
    Filesystem {
        #[serde(rename = "filesystem")]
        filesystem: FilesystemConfig,
    },
}

// Manual implementation to handle the nested structure
impl BackendConfig {
    pub fn as_reductstore(&self) -> Option<&ReductStoreConfig> {
        match self {
            BackendConfig::ReductStore { reductstore } => Some(reductstore),
            _ => None,
        }
    }
    
    pub fn as_reductstore_mut(&mut self) -> Option<&mut ReductStoreConfig> {
        match self {
            BackendConfig::ReductStore { reductstore } => Some(reductstore),
            _ => None,
        }
    }
    
    pub fn as_filesystem(&self) -> Option<&FilesystemConfig> {
        match self {
            BackendConfig::Filesystem { filesystem } => Some(filesystem),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReductStoreConfig {
    pub url: String,
    pub bucket_name: String,
    #[serde(default)]
    pub api_token: Option<String>,
    
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    
    #[serde(default = "default_retries")]
    pub max_retries: u32,
}

impl Default for ReductStoreConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:8383".to_string(),
            bucket_name: "zenoh_recordings".to_string(),
            api_token: None,
            timeout_seconds: default_timeout(),
            max_retries: default_retries(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FilesystemConfig {
    pub base_path: String,
    #[serde(default = "default_file_format")]
    pub file_format: String,  // "mcap"
}

impl Default for FilesystemConfig {
    fn default() -> Self {
        Self {
            base_path: "/data/recordings".to_string(),
            file_format: default_file_format(),
        }
    }
}

/// Recorder-specific settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecorderSettings {
    pub device_id: String,
    pub flush_policy: FlushPolicy,
    pub compression: CompressionConfig,
    #[serde(default)]
    pub workers: WorkerConfig,
    #[serde(default)]
    pub control: ControlConfig,
    #[serde(default)]
    pub schema: SchemaConfig,
}

impl Default for RecorderSettings {
    fn default() -> Self {
        Self {
            device_id: "recorder-001".to_string(),
            flush_policy: FlushPolicy::default(),
            compression: CompressionConfig::default(),
            workers: WorkerConfig::default(),
            control: ControlConfig::default(),
            schema: SchemaConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FlushPolicy {
    /// Maximum buffer size in bytes before flush
    pub max_buffer_size_bytes: usize,
    
    /// Maximum duration in seconds before flush
    pub max_buffer_duration_seconds: u64,
    
    /// Minimum samples before flush (avoid tiny flushes)
    #[serde(default = "default_min_samples")]
    pub min_samples_per_flush: usize,
}

impl Default for FlushPolicy {
    fn default() -> Self {
        Self {
            max_buffer_size_bytes: 10485760,  // 10 MB
            max_buffer_duration_seconds: 10,  // 10 seconds
            min_samples_per_flush: default_min_samples(),
        }
    }
}

impl FlushPolicy {
    pub fn max_duration(&self) -> Duration {
        Duration::from_secs(self.max_buffer_duration_seconds)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompressionConfig {
    pub default_type: String,  // "none", "lz4", "zstd"
    pub default_level: u8,     // 0-4
    
    #[serde(default)]
    pub per_topic: HashMap<String, TopicCompression>,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            default_type: "zstd".to_string(),
            default_level: 2,
            per_topic: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TopicCompression {
    pub r#type: String,
    pub level: u8,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchemaConfig {
    /// Default format for messages without explicit schema
    #[serde(default = "default_schema_format")]
    pub default_format: String,  // "raw", "protobuf", "json", etc.
    
    /// Whether to include schema metadata in recordings
    #[serde(default)]
    pub include_metadata: bool,
    
    /// Per-topic schema information
    #[serde(default)]
    pub per_topic: HashMap<String, TopicSchemaInfo>,
}

impl Default for SchemaConfig {
    fn default() -> Self {
        Self {
            default_format: default_schema_format(),
            include_metadata: false,
            per_topic: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TopicSchemaInfo {
    pub format: String,              // "protobuf", "json", "msgpack", "raw"
    #[serde(default)]
    pub schema_name: Option<String>, // e.g., "sensor_msgs/Image"
    #[serde(default)]
    pub schema_hash: Option<String>, // Optional version hash
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkerConfig {
    #[serde(default = "default_flush_workers")]
    pub flush_workers: usize,
    
    #[serde(default = "default_queue_capacity")]
    pub queue_capacity: usize,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            flush_workers: default_flush_workers(),
            queue_capacity: default_queue_capacity(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ControlConfig {
    #[serde(default = "default_control_prefix")]
    pub key_prefix: String,
    
    #[serde(default = "default_status_key")]
    pub status_key: String,
    
    #[serde(default = "default_control_timeout")]
    pub timeout_seconds: u64,
}

impl Default for ControlConfig {
    fn default() -> Self {
        Self {
            key_prefix: default_control_prefix(),
            status_key: default_status_key(),
            timeout_seconds: default_control_timeout(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,  // "trace", "debug", "info", "warn", "error"
    
    #[serde(default = "default_log_format")]
    pub format: String,  // "text", "json"
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
        }
    }
}

// Default value functions
fn default_mode() -> String { "peer".to_string() }
fn default_timeout() -> u64 { 300 }
fn default_retries() -> u32 { 3 }
fn default_min_samples() -> usize { 10 }
fn default_flush_workers() -> usize { 4 }
fn default_queue_capacity() -> usize { 1000 }
fn default_control_prefix() -> String { "recorder/control".to_string() }
fn default_status_key() -> String { "recorder/status/**".to_string() }
fn default_control_timeout() -> u64 { 30 }
fn default_log_level() -> String { "info".to_string() }
fn default_log_format() -> String { "text".to_string() }
fn default_file_format() -> String { "mcap".to_string() }
fn default_schema_format() -> String { "raw".to_string() }

