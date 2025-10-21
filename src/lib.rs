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

// Production-ready Zenoh Recorder with ReductStore Backend
//
// This is a high-performance data recorder for Zenoh middleware that:
// - Records multi-topic data streams
// - Flushes data based on size or time thresholds
// - Serializes to MCAP format with protobuf messages
// - Stores in ReductStore with configurable compression
// - Supports distributed recording control via request-response protocol

pub mod buffer;
pub mod config;
pub mod control;
pub mod mcap_writer;
pub mod protocol;
pub mod recorder;
pub mod storage;

// Re-export main types
pub use buffer::{FlushTask, TopicBuffer};
pub use config::{load_config, load_config_with_env, RecorderConfig};
pub use control::ControlInterface;
pub use mcap_writer::McapSerializer;
pub use protocol::{
    CompressionLevel, CompressionType, RecorderCommand, RecorderRequest, RecorderResponse,
    RecordingMetadata, RecordingStatus, StatusResponse,
};
pub use recorder::{RecorderManager, RecordingSession};
pub use storage::{topic_to_entry_name, ReductStoreClient};

// Include protobuf definitions
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/sensor_data.rs"));
}
