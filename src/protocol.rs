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

use serde::{Deserialize, Serialize};

/// Command types for recorder control
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecorderCommand {
    Start,
    Pause,
    Resume,
    Cancel,
    Finish,
}

/// Compression level (0-4)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum CompressionLevel {
    Fastest = 0,
    Fast = 1,
    #[default]
    Default = 2,
    Slow = 3,
    Slowest = 4,
}

impl CompressionLevel {
    pub fn to_zstd_level(self) -> i32 {
        match self {
            CompressionLevel::Fastest => 1,
            CompressionLevel::Fast => 3,
            CompressionLevel::Default => 5,
            CompressionLevel::Slow => 10,
            CompressionLevel::Slowest => 19,
        }
    }

    pub fn to_lz4_level(self) -> u32 {
        match self {
            CompressionLevel::Fastest => 1,
            CompressionLevel::Fast => 3,
            CompressionLevel::Default => 5,
            CompressionLevel::Slow => 9,
            CompressionLevel::Slowest => 12,
        }
    }
}

/// Compression type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CompressionType {
    None,
    Lz4,
    #[default]
    Zstd,
}

/// Request message for recording control operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecorderRequest {
    pub command: RecorderCommand,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    pub device_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_collector_id: Option<String>,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub compression_level: CompressionLevel,
    #[serde(default)]
    pub compression_type: CompressionType,
}

/// Response message for recording control operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecorderResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket_name: Option<String>,
}

/// Recording status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RecordingStatus {
    Idle,
    Recording,
    Paused,
    Uploading,
    Finished,
    Cancelled,
}

/// Response message for recording status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub success: bool,
    pub message: String,
    pub status: RecordingStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scene: Option<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    pub device_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_collector_id: Option<String>,
    #[serde(default)]
    pub active_topics: Vec<String>,
    pub buffer_size_bytes: i32,
    pub total_recorded_bytes: i64,
}

impl RecorderResponse {
    pub fn success(recording_id: Option<String>, bucket_name: Option<String>) -> Self {
        Self {
            success: true,
            message: "Operation completed successfully".to_string(),
            recording_id,
            bucket_name,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            message,
            recording_id: None,
            bucket_name: None,
        }
    }
}

/// Recording metadata stored in ReductStore
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingMetadata {
    pub recording_id: String,
    pub scene: Option<String>,
    pub skills: Vec<String>,
    pub organization: Option<String>,
    pub task_id: Option<String>,
    pub device_id: String,
    pub data_collector_id: Option<String>,
    pub topics: Vec<String>,
    pub compression_type: String,
    pub compression_level: i32,
    pub start_time: String,
    pub end_time: Option<String>,
    pub total_bytes: i64,
    pub total_samples: i64,
    pub per_topic_stats: serde_json::Value,
}
