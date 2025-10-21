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

/// Edge case and error path tests
///
use std::sync::Arc;
use std::time::Duration;
use zenoh::key_expr::KeyExpr;
use zenoh::sample::Sample;
use zenoh::Config;
use zenoh::Wait;
use zenoh_recorder::config::{BackendConfig, RecorderConfig, ReductStoreConfig, StorageConfig};
use zenoh_recorder::mcap_writer::McapSerializer;
use zenoh_recorder::protocol::*;
use zenoh_recorder::recorder::RecorderManager;
use zenoh_recorder::storage::BackendFactory;

fn create_sample(topic: &'static str, data: Vec<u8>) -> Sample {
    use zenoh::sample::SampleBuilder;
    let key: KeyExpr<'static> = topic.try_into().unwrap();
    SampleBuilder::put(key, data).into()
}

fn create_test_session() -> Result<Arc<zenoh::Session>, String> {
    let config = Config::default();
    zenoh::open(config)
        .wait()
        .map(Arc::new)
        .map_err(|e| format!("{}", e))
}

fn create_test_recorder_manager(
    session: Arc<zenoh::Session>,
    url: String,
    bucket: String,
) -> RecorderManager {
    let storage_config = StorageConfig {
        backend: "reductstore".to_string(),
        backend_config: BackendConfig::ReductStore {
            reductstore: ReductStoreConfig {
                url,
                bucket_name: bucket,
                api_token: None,
                timeout_seconds: 300,
                max_retries: 3,
            },
        },
    };

    let config = RecorderConfig {
        storage: storage_config,
        ..Default::default()
    };

    let storage_backend =
        BackendFactory::create(&config.storage).expect("Failed to create backend");

    RecorderManager::new(session, storage_backend, config)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_empty_topics_list() {
    let session = create_test_session().unwrap();
    let manager = create_test_recorder_manager(
        session,
        "http://localhost:8383".to_string(),
        "test_bucket".to_string(),
    );

    let request = RecorderRequest {
        command: RecorderCommand::Start,
        recording_id: None,
        scene: None,
        skills: vec![],
        organization: None,
        task_id: None,
        device_id: "device".to_string(),
        data_collector_id: None,
        topics: vec![], // Empty topics list
        compression_level: CompressionLevel::Default,
        compression_type: CompressionType::None,
    };

    let _response = manager.start_recording(request).await;
    // Should handle gracefully - may succeed or fail depending on environment
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_many_topics() {
    let session = create_test_session().unwrap();
    let manager = create_test_recorder_manager(
        session,
        "http://localhost:8383".to_string(),
        "test_bucket".to_string(),
    );

    // Create request with many topics
    let topics: Vec<String> = (0..50).map(|i| format!("test/topic{}", i)).collect();

    let request = RecorderRequest {
        command: RecorderCommand::Start,
        recording_id: None,
        scene: None,
        skills: vec![],
        organization: None,
        task_id: None,
        device_id: "device".to_string(),
        data_collector_id: None,
        topics,
        compression_level: CompressionLevel::Default,
        compression_type: CompressionType::None,
    };

    let response = manager.start_recording(request).await;

    if let Some(rec_id) = &response.recording_id {
        tokio::time::sleep(Duration::from_millis(100)).await;
        manager.cancel_recording(rec_id).await;
    }
}

#[test]
fn test_very_large_compression_level() {
    let serializer = McapSerializer::new(CompressionType::Zstd, CompressionLevel::Slowest);
    let sample = create_sample("test/topic", b"data".to_vec());

    let result = serializer.serialize_batch("/test/topic", vec![sample], "rec-123");
    assert!(result.is_ok());
}

#[test]
fn test_serialization_with_empty_recording_id() {
    let serializer = McapSerializer::new(CompressionType::None, CompressionLevel::Default);
    let sample = create_sample("test/topic", b"data".to_vec());

    let result = serializer.serialize_batch("/test/topic", vec![sample], "");
    assert!(result.is_ok());
}

#[test]
fn test_serialization_with_special_chars_in_topic() {
    let serializer = McapSerializer::new(CompressionType::None, CompressionLevel::Default);
    let sample = create_sample("test/topic", b"data".to_vec());

    // Topic with special characters in path
    let result = serializer.serialize_batch("/test-topic_with.chars", vec![sample], "rec-123");
    assert!(result.is_ok());
}

#[test]
fn test_all_compression_combinations() {
    let compression_types = vec![
        CompressionType::None,
        CompressionType::Lz4,
        CompressionType::Zstd,
    ];

    let compression_levels = vec![
        CompressionLevel::Fastest,
        CompressionLevel::Fast,
        CompressionLevel::Default,
        CompressionLevel::Slow,
        CompressionLevel::Slowest,
    ];

    let sample = create_sample("test/topic", b"test data for compression".to_vec());

    for comp_type in &compression_types {
        for comp_level in &compression_levels {
            let serializer = McapSerializer::new(*comp_type, *comp_level);
            let result = serializer.serialize_batch("/test/topic", vec![sample.clone()], "rec-123");
            assert!(
                result.is_ok(),
                "Failed for {:?} at {:?}",
                comp_type,
                comp_level
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rapid_start_stop() {
    let session = create_test_session().unwrap();
    let manager = create_test_recorder_manager(
        session,
        "http://localhost:8383".to_string(),
        "test_bucket".to_string(),
    );

    // Rapidly start and stop recordings
    for i in 0..5 {
        let request = RecorderRequest {
            command: RecorderCommand::Start,
            recording_id: None,
            scene: None,
            skills: vec![],
            organization: None,
            task_id: None,
            device_id: format!("device-{}", i),
            data_collector_id: None,
            topics: vec![format!("test/rapid{}", i)],
            compression_level: CompressionLevel::Fastest,
            compression_type: CompressionType::None,
        };

        let response = manager.start_recording(request).await;

        if let Some(rec_id) = &response.recording_id {
            // Immediately cancel
            manager.cancel_recording(rec_id).await;
        }
    }
}

#[test]
fn test_recorder_response_builder_functions() {
    // Test success builder
    let success_resp =
        RecorderResponse::success(Some("rec-123".to_string()), Some("bucket".to_string()));
    assert!(success_resp.success);
    assert_eq!(success_resp.message, "Operation completed successfully");
    assert_eq!(success_resp.recording_id, Some("rec-123".to_string()));
    assert_eq!(success_resp.bucket_name, Some("bucket".to_string()));

    // Test error builder
    let error_resp = RecorderResponse::error("Test error message".to_string());
    assert!(!error_resp.success);
    assert_eq!(error_resp.message, "Test error message");
    assert!(error_resp.recording_id.is_none());
    assert!(error_resp.bucket_name.is_none());
}

#[test]
fn test_request_with_minimal_fields() {
    let request = RecorderRequest {
        command: RecorderCommand::Start,
        recording_id: None,
        scene: None,
        skills: vec![],
        organization: None,
        task_id: None,
        device_id: "minimal-device".to_string(),
        data_collector_id: None,
        topics: vec![],
        compression_level: CompressionLevel::Default,
        compression_type: CompressionType::Zstd,
    };

    let json = serde_json::to_string(&request).unwrap();
    let deserialized: RecorderRequest = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.device_id, "minimal-device");
    assert!(deserialized.scene.is_none());
    assert!(deserialized.topics.is_empty());
}

#[test]
fn test_request_with_maximal_fields() {
    let request = RecorderRequest {
        command: RecorderCommand::Start,
        recording_id: Some("pre-assigned-id".to_string()),
        scene: Some("scene".to_string()),
        skills: vec!["s1".to_string(), "s2".to_string(), "s3".to_string()],
        organization: Some("org".to_string()),
        task_id: Some("task".to_string()),
        device_id: "device".to_string(),
        data_collector_id: Some("collector".to_string()),
        topics: vec!["t1".to_string(), "t2".to_string()],
        compression_level: CompressionLevel::Slowest,
        compression_type: CompressionType::Lz4,
    };

    let json = serde_json::to_string(&request).unwrap();
    let deserialized: RecorderRequest = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.skills.len(), 3);
    assert_eq!(deserialized.topics.len(), 2);
    assert!(deserialized.recording_id.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_finish_immediately_after_start() {
    let session = create_test_session().unwrap();
    let manager = create_test_recorder_manager(
        session,
        "http://localhost:8383".to_string(),
        "test_bucket".to_string(),
    );

    let request = RecorderRequest {
        command: RecorderCommand::Start,
        recording_id: None,
        scene: None,
        skills: vec![],
        organization: None,
        task_id: None,
        device_id: "device".to_string(),
        data_collector_id: None,
        topics: vec!["test/immediate".to_string()],
        compression_level: CompressionLevel::Default,
        compression_type: CompressionType::None,
    };

    let response = manager.start_recording(request).await;

    if let Some(rec_id) = &response.recording_id {
        // Finish immediately without pause
        let _finish_resp = manager.finish_recording(rec_id).await;
        // May succeed or fail depending on recording state
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_pause_without_recording() {
    let session = create_test_session().unwrap();
    let manager = create_test_recorder_manager(
        session,
        "http://localhost:8383".to_string(),
        "test_bucket".to_string(),
    );

    let response = manager.pause_recording("never-started").await;
    assert!(!response.success);
    assert!(response.message.to_lowercase().contains("not found"));
}

#[test]
fn test_status_response_with_large_values() {
    let response = StatusResponse {
        success: true,
        message: "OK".to_string(),
        status: RecordingStatus::Recording,
        scene: Some("test".to_string()),
        skills: vec!["skill".to_string(); 100], // 100 skills
        organization: Some("org".to_string()),
        task_id: Some("task".to_string()),
        device_id: "device".to_string(),
        data_collector_id: Some("collector".to_string()),
        active_topics: (0..50).map(|i| format!("/topic{}", i)).collect(), // 50 topics
        buffer_size_bytes: i32::MAX,
        total_recorded_bytes: i64::MAX,
    };

    assert_eq!(response.skills.len(), 100);
    assert_eq!(response.active_topics.len(), 50);
    assert_eq!(response.buffer_size_bytes, i32::MAX);
    assert_eq!(response.total_recorded_bytes, i64::MAX);
}
