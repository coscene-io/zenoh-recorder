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

use zenoh_recorder::protocol::*;

#[test]
fn test_compression_level_defaults() {
    let level = CompressionLevel::default();
    assert_eq!(level as u8, 2);
}

#[test]
fn test_compression_level_to_zstd() {
    assert_eq!(CompressionLevel::Fastest.to_zstd_level(), 1);
    assert_eq!(CompressionLevel::Fast.to_zstd_level(), 3);
    assert_eq!(CompressionLevel::Default.to_zstd_level(), 5);
    assert_eq!(CompressionLevel::Slow.to_zstd_level(), 10);
    assert_eq!(CompressionLevel::Slowest.to_zstd_level(), 19);
}

#[test]
fn test_compression_level_to_lz4() {
    assert_eq!(CompressionLevel::Fastest.to_lz4_level(), 1);
    assert_eq!(CompressionLevel::Fast.to_lz4_level(), 3);
    assert_eq!(CompressionLevel::Default.to_lz4_level(), 5);
    assert_eq!(CompressionLevel::Slow.to_lz4_level(), 9);
    assert_eq!(CompressionLevel::Slowest.to_lz4_level(), 12);
}

#[test]
fn test_compression_type_default() {
    let comp_type = CompressionType::default();
    assert_eq!(comp_type, CompressionType::Zstd);
}

#[test]
fn test_recorder_request_serialization() {
    let request = RecorderRequest {
        command: RecorderCommand::Start,
        recording_id: Some("test-123".to_string()),
        scene: Some("test_scene".to_string()),
        skills: vec!["skill1".to_string(), "skill2".to_string()],
        organization: Some("test_org".to_string()),
        task_id: Some("task-001".to_string()),
        device_id: "device-01".to_string(),
        data_collector_id: Some("collector-01".to_string()),
        topics: vec!["/test/topic1".to_string()],
        compression_level: CompressionLevel::Default,
        compression_type: CompressionType::Zstd,
    };

    let json = serde_json::to_string(&request).unwrap();
    let deserialized: RecorderRequest = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.device_id, "device-01");
    assert_eq!(deserialized.topics.len(), 1);
}

#[test]
fn test_recorder_response_success() {
    let response =
        RecorderResponse::success(Some("rec-123".to_string()), Some("bucket".to_string()));

    assert!(response.success);
    assert_eq!(response.recording_id, Some("rec-123".to_string()));
    assert_eq!(response.bucket_name, Some("bucket".to_string()));
}

#[test]
fn test_recorder_response_error() {
    let response = RecorderResponse::error("Test error".to_string());

    assert!(!response.success);
    assert_eq!(response.message, "Test error");
    assert!(response.recording_id.is_none());
    assert!(response.bucket_name.is_none());
}

#[test]
fn test_recording_status_serialization() {
    let status = RecordingStatus::Recording;
    let json = serde_json::to_string(&status).unwrap();
    assert_eq!(json, "\"recording\"");

    let deserialized: RecordingStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, RecordingStatus::Recording);
}

#[test]
fn test_status_response() {
    let response = StatusResponse {
        success: true,
        message: "OK".to_string(),
        status: RecordingStatus::Recording,
        scene: Some("test".to_string()),
        skills: vec!["skill1".to_string()],
        organization: Some("org".to_string()),
        task_id: Some("task".to_string()),
        device_id: "device".to_string(),
        data_collector_id: Some("collector".to_string()),
        active_topics: vec!["/topic1".to_string()],
        buffer_size_bytes: 1024,
        total_recorded_bytes: 4096,
    };

    assert!(response.success);
    assert_eq!(response.buffer_size_bytes, 1024);
    assert_eq!(response.total_recorded_bytes, 4096);
}
