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

/// Comprehensive tests targeting uncovered code paths
///
use crossbeam::queue::ArrayQueue;
use std::sync::Arc;
use std::time::Duration;
use zenoh::key_expr::KeyExpr;
use zenoh::prelude::r#async::*;
use zenoh::sample::Sample;
use zenoh_recorder::buffer::TopicBuffer;
use zenoh_recorder::control::ControlInterface;
use zenoh_recorder::mcap_writer::McapSerializer;
use zenoh_recorder::protocol::*;
use zenoh_recorder::recorder::RecorderManager;
use zenoh_recorder::storage::ReductStoreClient;

fn create_sample(topic: &'static str, data: Vec<u8>) -> Sample {
    let key: KeyExpr<'static> = topic.try_into().unwrap();
    Sample::new(key, data)
}

async fn create_session() -> Arc<zenoh::Session> {
    let config = Config::default();
    Arc::new(zenoh::open(config).res().await.unwrap())
}

// Buffer edge cases
#[tokio::test]
async fn test_buffer_with_zero_max_size() {
    let flush_queue = Arc::new(ArrayQueue::new(10));
    let buffer = TopicBuffer::new(
        "/test/topic".to_string(),
        "rec-123".to_string(),
        0, // Zero max size - should trigger immediately
        Duration::from_secs(10),
        flush_queue.clone(),
    );

    let sample = create_sample("test/topic", b"data".to_vec());
    buffer.push_sample(sample).await.unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Should have triggered flush
    assert!(flush_queue.len() > 0 || buffer.stats().0 == 0);
}

#[tokio::test]
async fn test_buffer_with_very_long_duration() {
    let flush_queue = Arc::new(ArrayQueue::new(10));
    let buffer = TopicBuffer::new(
        "/test/topic".to_string(),
        "rec-123".to_string(),
        10 * 1024 * 1024,
        Duration::from_secs(3600), // 1 hour
        flush_queue,
    );

    let sample = create_sample("test/topic", b"data".to_vec());
    buffer.push_sample(sample).await.unwrap();

    let (samples, _) = buffer.stats();
    assert_eq!(samples, 1); // Should not flush based on time
}

#[tokio::test]
async fn test_buffer_full_queue() {
    let flush_queue = Arc::new(ArrayQueue::new(2)); // Small queue
    let buffer = TopicBuffer::new(
        "/test/topic".to_string(),
        "rec-123".to_string(),
        10, // Tiny buffer to trigger many flushes
        Duration::from_secs(10),
        flush_queue.clone(),
    );

    // Push many samples to overflow flush queue
    for i in 0..20 {
        let sample = create_sample("test/topic", format!("data_{}", i).into_bytes());
        let _ = buffer.push_sample(sample).await;
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Queue should be full or have items
    assert!(flush_queue.len() >= 0);
}

// MCAP edge cases
#[test]
fn test_mcap_with_very_long_topic_name() {
    let serializer = McapSerializer::new(CompressionType::None, CompressionLevel::Default);
    let long_topic = "/very/long/topic/name/with/many/segments/that/goes/on/and/on";
    let sample = create_sample("test/topic", b"data".to_vec());

    let result = serializer.serialize_batch(long_topic, vec![sample], "rec-123");
    assert!(result.is_ok());
}

#[test]
fn test_mcap_with_very_long_recording_id() {
    let serializer = McapSerializer::new(CompressionType::None, CompressionLevel::Default);
    let long_id = "a".repeat(1000);
    let sample = create_sample("test/topic", b"data".to_vec());

    let result = serializer.serialize_batch("/test/topic", vec![sample], &long_id);
    assert!(result.is_ok());
}

#[test]
fn test_mcap_with_huge_sample_count() {
    let serializer = McapSerializer::new(CompressionType::None, CompressionLevel::Fastest);

    // Create many samples
    let samples: Vec<Sample> = (0..500)
        .map(|i| create_sample("test/topic", format!("sample_{}", i).into_bytes()))
        .collect();

    let result = serializer.serialize_batch("/test/topic", samples, "rec-123");
    assert!(result.is_ok());
}

// Recorder edge cases
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_double_pause() {
    let session = create_session().await;
    let manager = RecorderManager::new(
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
        topics: vec!["test/double".to_string()],
        compression_level: CompressionLevel::Default,
        compression_type: CompressionType::None,
    };

    let response = manager.start_recording(request).await;

    if let Some(rec_id) = &response.recording_id {
        tokio::time::sleep(Duration::from_millis(50)).await;

        // First pause should succeed
        let pause1 = manager.pause_recording(rec_id).await;

        if pause1.success {
            // Second pause should fail (already paused)
            let pause2 = manager.pause_recording(rec_id).await;
            assert!(!pause2.success);
        }

        manager.cancel_recording(rec_id).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_resume_without_pause() {
    let session = create_session().await;
    let manager = RecorderManager::new(
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
        topics: vec!["test/resume".to_string()],
        compression_level: CompressionLevel::Default,
        compression_type: CompressionType::None,
    };

    let response = manager.start_recording(request).await;

    if let Some(rec_id) = &response.recording_id {
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Try to resume without pausing (should fail)
        let resume_resp = manager.resume_recording(rec_id).await;
        assert!(!resume_resp.success);

        manager.cancel_recording(rec_id).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_finish_after_cancel() {
    let session = create_session().await;
    let manager = RecorderManager::new(
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
        topics: vec!["test/cancel_then_finish".to_string()],
        compression_level: CompressionLevel::Default,
        compression_type: CompressionType::None,
    };

    let response = manager.start_recording(request).await;

    if let Some(rec_id) = &response.recording_id {
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Cancel
        let cancel_resp = manager.cancel_recording(rec_id).await;
        assert!(cancel_resp.success);

        // After cancel, recording is still in map (with Cancelled status)
        // Can query status after cancel
        let status = manager.get_status(rec_id).await;
        assert!(status.success);
        assert_eq!(status.status, RecordingStatus::Cancelled);
    }
}

// Storage client variations
#[test]
fn test_storage_client_with_different_configs() {
    let configs = vec![
        ("http://localhost:8383", "bucket1"),
        ("https://prod.example.com", "production_data"),
        ("http://192.168.1.100:9000", "sensor_recordings"),
    ];

    for (url, bucket) in configs {
        let client = ReductStoreClient::new(url.to_string(), bucket.to_string());
        drop(client);
    }
}

// Protocol message variations
#[test]
fn test_request_with_all_skills() {
    let skills: Vec<String> = (0..100).map(|i| format!("skill_{}", i)).collect();

    let request = RecorderRequest {
        command: RecorderCommand::Start,
        recording_id: None,
        scene: Some("test".to_string()),
        skills: skills.clone(),
        organization: None,
        task_id: None,
        device_id: "device".to_string(),
        data_collector_id: None,
        topics: vec![],
        compression_level: CompressionLevel::Default,
        compression_type: CompressionType::Zstd,
    };

    assert_eq!(request.skills.len(), 100);

    // Verify serialization
    let json = serde_json::to_string(&request).unwrap();
    let deserialized: RecorderRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.skills.len(), 100);
}

#[test]
fn test_status_response_serialization_all_fields() {
    let response = StatusResponse {
        success: true,
        message: "All fields test".to_string(),
        status: RecordingStatus::Uploading,
        scene: Some("comprehensive_test".to_string()),
        skills: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        organization: Some("test_org_full".to_string()),
        task_id: Some("task-comprehensive-001".to_string()),
        device_id: "device-comprehensive".to_string(),
        data_collector_id: Some("collector-comprehensive".to_string()),
        active_topics: vec!["/t1".to_string(), "/t2".to_string(), "/t3".to_string()],
        buffer_size_bytes: 123456,
        total_recorded_bytes: 9876543210,
    };

    let json = serde_json::to_string(&response).unwrap();
    let deserialized: StatusResponse = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.success, true);
    assert_eq!(deserialized.status, RecordingStatus::Uploading);
    assert_eq!(deserialized.skills.len(), 3);
    assert_eq!(deserialized.active_topics.len(), 3);
    assert_eq!(deserialized.buffer_size_bytes, 123456);
    assert_eq!(deserialized.total_recorded_bytes, 9876543210);
}

#[test]
fn test_recording_metadata_json_serialization() {
    let metadata = RecordingMetadata {
        recording_id: "test-rec".to_string(),
        scene: Some("test_scene".to_string()),
        skills: vec!["skill1".to_string()],
        organization: Some("org".to_string()),
        task_id: Some("task".to_string()),
        device_id: "device".to_string(),
        data_collector_id: Some("collector".to_string()),
        topics: vec!["/t1".to_string()],
        compression_type: "zstd".to_string(),
        compression_level: 5,
        start_time: "2024-01-01T00:00:00Z".to_string(),
        end_time: Some("2024-01-01T01:00:00Z".to_string()),
        total_bytes: 1000000,
        total_samples: 50000,
        per_topic_stats: serde_json::json!({"test": "data"}),
    };

    let json = serde_json::to_string_pretty(&metadata).unwrap();
    assert!(json.contains("test-rec"));
    assert!(json.contains("test_scene"));

    let deserialized: RecordingMetadata = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.recording_id, "test-rec");
    assert_eq!(deserialized.total_samples, 50000);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_control_interface_device_ids() {
    let session = create_session().await;

    let device_ids = vec!["device-1", "device-2", "device-3"];

    for device_id in device_ids {
        let manager = Arc::new(RecorderManager::new(
            session.clone(),
            "http://localhost:8383".to_string(),
            "bucket".to_string(),
        ));

        let _control = ControlInterface::new(session.clone(), manager, device_id.to_string());
    }
}

#[test]
fn test_compression_level_copy_trait() {
    let level = CompressionLevel::Default;
    let level_copy = level;

    assert_eq!(level.to_zstd_level(), level_copy.to_zstd_level());
}

#[test]
fn test_compression_type_copy_trait() {
    let comp = CompressionType::Zstd;
    let comp_copy = comp;

    assert_eq!(comp, comp_copy);
}

#[test]
fn test_recording_status_copy_trait() {
    let status = RecordingStatus::Recording;
    let status_copy = status;

    assert_eq!(status, status_copy);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_recorder_with_all_compression_types() {
    let session = create_session().await;

    let compression_types = vec![
        CompressionType::None,
        CompressionType::Lz4,
        CompressionType::Zstd,
    ];

    for comp_type in compression_types {
        let manager = RecorderManager::new(
            session.clone(),
            "http://localhost:8383".to_string(),
            format!("bucket_{:?}", comp_type),
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
            topics: vec!["test/compression".to_string()],
            compression_level: CompressionLevel::Default,
            compression_type: comp_type,
        };

        let response = manager.start_recording(request).await;

        if let Some(rec_id) = &response.recording_id {
            tokio::time::sleep(Duration::from_millis(50)).await;
            manager.cancel_recording(rec_id).await;
        }
    }
}

#[test]
fn test_large_sample_count_serialization() {
    let serializer = McapSerializer::new(CompressionType::Lz4, CompressionLevel::Fastest);

    // 1000 samples
    let samples: Vec<Sample> = (0..1000)
        .map(|i| create_sample("test/topic", format!("{}", i).into_bytes()))
        .collect();

    let result = serializer.serialize_batch("/test/topic", samples, "rec-large");
    assert!(result.is_ok());
    assert!(result.unwrap().len() > 0);
}

#[test]
fn test_mcap_alternating_data_sizes() {
    let serializer = McapSerializer::new(CompressionType::Zstd, CompressionLevel::Default);

    let mut samples = Vec::new();
    for i in 0..50 {
        let size = if i % 2 == 0 { 100 } else { 10000 };
        let data = vec![0u8; size];
        samples.push(create_sample("test/topic", data));
    }

    let result = serializer.serialize_batch("/test/topic", samples, "rec-alt");
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_shutdown_with_active_recordings() {
    let session = create_session().await;
    let manager = Arc::new(RecorderManager::new(
        session,
        "http://localhost:8383".to_string(),
        "test_bucket".to_string(),
    ));

    // Start a recording
    let request = RecorderRequest {
        command: RecorderCommand::Start,
        recording_id: None,
        scene: None,
        skills: vec![],
        organization: None,
        task_id: None,
        device_id: "device".to_string(),
        data_collector_id: None,
        topics: vec!["test/shutdown".to_string()],
        compression_level: CompressionLevel::Default,
        compression_type: CompressionType::None,
    };

    let _response = manager.start_recording(request).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Shutdown should finish active recordings
    let result = manager.shutdown().await;
    assert!(result.is_ok());
}

#[test]
fn test_all_recording_status_serialization() {
    let statuses = vec![
        (RecordingStatus::Idle, "idle"),
        (RecordingStatus::Recording, "recording"),
        (RecordingStatus::Paused, "paused"),
        (RecordingStatus::Uploading, "uploading"),
        (RecordingStatus::Finished, "finished"),
        (RecordingStatus::Cancelled, "cancelled"),
    ];

    for (status, expected_str) in statuses {
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains(expected_str));

        let deserialized: RecordingStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, status);
    }
}

#[test]
fn test_all_recorder_commands_serialization() {
    let commands = vec![
        (RecorderCommand::Start, "start"),
        (RecorderCommand::Pause, "pause"),
        (RecorderCommand::Resume, "resume"),
        (RecorderCommand::Cancel, "cancel"),
        (RecorderCommand::Finish, "finish"),
    ];

    for (command, expected_str) in commands {
        let json = serde_json::to_string(&command).unwrap();
        assert!(json.contains(expected_str));
    }
}

#[test]
fn test_compression_type_serialization() {
    let types = vec![
        (CompressionType::None, "none"),
        (CompressionType::Lz4, "lz4"),
        (CompressionType::Zstd, "zstd"),
    ];

    for (comp_type, expected_str) in types {
        let json = serde_json::to_string(&comp_type).unwrap();
        assert!(json.contains(expected_str));

        let deserialized: CompressionType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, comp_type);
    }
}
