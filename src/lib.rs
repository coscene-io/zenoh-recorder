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
pub use config::{RecorderConfig, load_config, load_config_with_env};
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
