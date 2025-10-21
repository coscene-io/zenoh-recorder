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

// Unit tests for control.rs module - mock-based tests without requiring Zenoh infrastructure
use serde_json;
use zenoh_recorder::protocol::{
    CompressionLevel, CompressionType, RecorderCommand, RecorderRequest, RecorderResponse,
    RecordingStatus, StatusResponse,
};

#[test]
fn test_control_request_parsing_start_command() {
    let request = RecorderRequest {
        command: RecorderCommand::Start,
        recording_id: Some("test-001".to_string()),
        topics: vec!["topic1".to_string(), "topic2".to_string()],
        scene: Some("test_scene".to_string()),
        skills: vec!["skill1".to_string()],
        organization: Some("test_org".to_string()),
        task_id: Some("task-123".to_string()),
        device_id: "device-456".to_string(),
        data_collector_id: Some("collector-789".to_string()),
        compression_type: CompressionType::Zstd,
        compression_level: CompressionLevel::Default,
    };

    // Serialize and deserialize
    let json = serde_json::to_string(&request).unwrap();
    let parsed: RecorderRequest = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.recording_id, Some("test-001".to_string()));
    assert_eq!(parsed.topics.len(), 2);
    assert_eq!(parsed.scene, Some("test_scene".to_string()));
    assert_eq!(parsed.compression_type, CompressionType::Zstd);
}

#[test]
fn test_control_request_parsing_pause_command() {
    let request = RecorderRequest {
        command: RecorderCommand::Pause,
        recording_id: Some("rec-001".to_string()),
        topics: vec![],
        scene: None,
        skills: vec![],
        organization: None,
        task_id: None,
        device_id: String::new(),
        data_collector_id: None,
        compression_type: CompressionType::default(),
        compression_level: CompressionLevel::default(),
    };

    let json = serde_json::to_string(&request).unwrap();
    let parsed: RecorderRequest = serde_json::from_str(&json).unwrap();

    assert!(matches!(parsed.command, RecorderCommand::Pause));
    assert_eq!(parsed.recording_id, Some("rec-001".to_string()));
}

#[test]
fn test_control_request_parsing_resume_command() {
    let request = RecorderRequest {
        command: RecorderCommand::Resume,
        recording_id: Some("rec-002".to_string()),
        topics: vec![],
        scene: None,
        skills: vec![],
        organization: None,
        task_id: None,
        device_id: String::new(),
        data_collector_id: None,
        compression_type: CompressionType::default(),
        compression_level: CompressionLevel::default(),
    };

    let json = serde_json::to_string(&request).unwrap();
    let parsed: RecorderRequest = serde_json::from_str(&json).unwrap();

    assert!(matches!(parsed.command, RecorderCommand::Resume));
    assert_eq!(parsed.recording_id, Some("rec-002".to_string()));
}

#[test]
fn test_control_request_parsing_cancel_command() {
    let request = RecorderRequest {
        command: RecorderCommand::Cancel,
        recording_id: Some("rec-003".to_string()),
        topics: vec![],
        scene: None,
        skills: vec![],
        organization: None,
        task_id: None,
        device_id: String::new(),
        data_collector_id: None,
        compression_type: CompressionType::default(),
        compression_level: CompressionLevel::default(),
    };

    let json = serde_json::to_string(&request).unwrap();
    let parsed: RecorderRequest = serde_json::from_str(&json).unwrap();

    assert!(matches!(parsed.command, RecorderCommand::Cancel));
}

#[test]
fn test_control_request_parsing_finish_command() {
    let request = RecorderRequest {
        command: RecorderCommand::Finish,
        recording_id: Some("rec-004".to_string()),
        topics: vec![],
        scene: None,
        skills: vec![],
        organization: None,
        task_id: None,
        device_id: String::new(),
        data_collector_id: None,
        compression_type: CompressionType::default(),
        compression_level: CompressionLevel::default(),
    };

    let json = serde_json::to_string(&request).unwrap();
    let parsed: RecorderRequest = serde_json::from_str(&json).unwrap();

    assert!(matches!(parsed.command, RecorderCommand::Finish));
}

#[test]
fn test_control_response_success() {
    let response = RecorderResponse::success(Some("rec-001".to_string()), Some("test-bucket".to_string()));

    let json = serde_json::to_string(&response).unwrap();
    let parsed: RecorderResponse = serde_json::from_str(&json).unwrap();

    assert!(parsed.success);
    assert_eq!(parsed.recording_id, Some("rec-001".to_string()));
    assert_eq!(parsed.bucket_name, Some("test-bucket".to_string()));
    assert!(parsed.message.contains("successfully") || parsed.message.contains("Success"));
}

#[test]
fn test_control_response_error() {
    let response = RecorderResponse::error("Test error message".to_string());

    let json = serde_json::to_string(&response).unwrap();
    let parsed: RecorderResponse = serde_json::from_str(&json).unwrap();

    assert!(!parsed.success);
    assert_eq!(parsed.message, "Test error message");
    assert_eq!(parsed.recording_id, None);
}

#[test]
fn test_status_response_serialization() {
    let response = StatusResponse {
        success: true,
        message: "Recording is active".to_string(),
        status: RecordingStatus::Recording,
        scene: Some("test_scene".to_string()),
        skills: vec!["skill1".to_string(), "skill2".to_string()],
        organization: Some("org".to_string()),
        task_id: Some("task-1".to_string()),
        device_id: "device-1".to_string(),
        data_collector_id: Some("collector-1".to_string()),
        active_topics: vec!["topic1".to_string(), "topic2".to_string()],
        buffer_size_bytes: 1024,
        total_recorded_bytes: 10240,
    };

    let json = serde_json::to_string(&response).unwrap();
    let parsed: StatusResponse = serde_json::from_str(&json).unwrap();

    assert!(parsed.success);
    assert_eq!(parsed.status, RecordingStatus::Recording);
    assert_eq!(parsed.active_topics.len(), 2);
    assert_eq!(parsed.buffer_size_bytes, 1024);
    assert_eq!(parsed.total_recorded_bytes, 10240);
}

#[test]
fn test_status_response_idle_state() {
    let response = StatusResponse {
        success: true,
        message: "No active recording".to_string(),
        status: RecordingStatus::Idle,
        scene: None,
        skills: vec![],
        organization: None,
        task_id: None,
        device_id: "device-1".to_string(),
        data_collector_id: None,
        active_topics: vec![],
        buffer_size_bytes: 0,
        total_recorded_bytes: 0,
    };

    let json = serde_json::to_string(&response).unwrap();
    let parsed: StatusResponse = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.status, RecordingStatus::Idle);
    assert_eq!(parsed.buffer_size_bytes, 0);
    assert_eq!(parsed.active_topics.len(), 0);
}

#[test]
fn test_status_response_paused_state() {
    let response = StatusResponse {
        success: true,
        message: "Recording paused".to_string(),
        status: RecordingStatus::Paused,
        scene: Some("scene1".to_string()),
        skills: vec!["skill1".to_string()],
        organization: Some("org".to_string()),
        task_id: Some("task-1".to_string()),
        device_id: "device-1".to_string(),
        data_collector_id: Some("collector-1".to_string()),
        active_topics: vec!["topic1".to_string()],
        buffer_size_bytes: 512,
        total_recorded_bytes: 5120,
    };

    let json = serde_json::to_string(&response).unwrap();
    let parsed: StatusResponse = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.status, RecordingStatus::Paused);
    assert!(parsed.buffer_size_bytes > 0);
}

#[test]
fn test_control_key_format_parsing() {
    // Test parsing of control key format: recorder/control/{device_id}
    let key = "recorder/control/device-123";
    let parts: Vec<&str> = key.split('/').collect();

    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], "recorder");
    assert_eq!(parts[1], "control");
    assert_eq!(parts[2], "device-123");
}

#[test]
fn test_status_key_format_parsing() {
    // Test parsing of status key format: recorder/status/{recording_id}
    let key = "recorder/status/rec-001";
    let parts: Vec<&str> = key.split('/').collect();

    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], "recorder");
    assert_eq!(parts[1], "status");
    assert_eq!(parts[2], "rec-001");
}

#[test]
fn test_status_key_format_invalid() {
    // Test invalid status key format
    let key = "recorder/status";
    let parts: Vec<&str> = key.split('/').collect();

    assert!(parts.len() < 3);
}

#[test]
fn test_request_with_empty_recording_id() {
    let request = RecorderRequest {
        command: RecorderCommand::Pause,
        recording_id: Some("".to_string()),
        topics: vec![],
        scene: None,
        skills: vec![],
        organization: None,
        task_id: None,
        device_id: String::new(),
        data_collector_id: None,
        compression_type: CompressionType::default(),
        compression_level: CompressionLevel::default(),
    };

    let json = serde_json::to_string(&request).unwrap();
    let parsed: RecorderRequest = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.recording_id, Some("".to_string()));
    assert_eq!(parsed.recording_id.unwrap_or_default(), "");
}

#[test]
fn test_request_with_none_recording_id() {
    let request = RecorderRequest {
        command: RecorderCommand::Cancel,
        recording_id: None,
        topics: vec![],
        scene: None,
        skills: vec![],
        organization: None,
        task_id: None,
        device_id: String::new(),
        data_collector_id: None,
        compression_type: CompressionType::default(),
        compression_level: CompressionLevel::default(),
    };

    let json = serde_json::to_string(&request).unwrap();
    let parsed: RecorderRequest = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.recording_id, None);
    assert_eq!(parsed.recording_id.unwrap_or_default(), "");
}

#[test]
fn test_response_json_structure() {
    let response = RecorderResponse::success(Some("test-id".to_string()), None);
    let json = serde_json::to_string(&response).unwrap();

    assert!(json.contains("success"));
    assert!(json.contains("message"));
    assert!(json.contains("recording_id"));
}

#[test]
fn test_status_response_json_structure() {
    let response = StatusResponse {
        success: true,
        message: "OK".to_string(),
        status: RecordingStatus::Recording,
        scene: None,
        skills: vec![],
        organization: None,
        task_id: None,
        device_id: "dev-1".to_string(),
        data_collector_id: None,
        active_topics: vec![],
        buffer_size_bytes: 0,
        total_recorded_bytes: 0,
    };

    let json = serde_json::to_string(&response).unwrap();

    assert!(json.contains("success"));
    assert!(json.contains("message"));
    assert!(json.contains("status"));
    assert!(json.contains("device_id"));
}

#[test]
fn test_request_with_all_commands() {
    let commands = vec![
        RecorderCommand::Start,
        RecorderCommand::Pause,
        RecorderCommand::Resume,
        RecorderCommand::Cancel,
        RecorderCommand::Finish,
    ];

    for command in commands {
        let request = RecorderRequest {
            command: command.clone(),
            recording_id: Some("test".to_string()),
            topics: vec![],
            scene: None,
            skills: vec![],
            organization: None,
            task_id: None,
            device_id: String::new(),
            data_collector_id: None,
            compression_type: CompressionType::default(),
            compression_level: CompressionLevel::default(),
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: RecorderRequest = serde_json::from_str(&json).unwrap();

        // Verify command roundtrip
        assert_eq!(
            std::mem::discriminant(&parsed.command),
            std::mem::discriminant(&command)
        );
    }
}

#[test]
fn test_status_with_large_buffer_size() {
    let response = StatusResponse {
        success: true,
        message: "OK".to_string(),
        status: RecordingStatus::Recording,
        scene: None,
        skills: vec![],
        organization: None,
        task_id: None,
        device_id: "dev-1".to_string(),
        data_collector_id: None,
        active_topics: vec![],
        buffer_size_bytes: 1_000_000_000, // 1GB
        total_recorded_bytes: 10_000_000_000, // 10GB
    };

    let json = serde_json::to_string(&response).unwrap();
    let parsed: StatusResponse = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.buffer_size_bytes, 1_000_000_000);
    assert_eq!(parsed.total_recorded_bytes, 10_000_000_000);
}

#[test]
fn test_status_with_many_topics() {
    let topics: Vec<String> = (0..100).map(|i| format!("topic_{}", i)).collect();

    let response = StatusResponse {
        success: true,
        message: "OK".to_string(),
        status: RecordingStatus::Recording,
        scene: None,
        skills: vec![],
        organization: None,
        task_id: None,
        device_id: "dev-1".to_string(),
        data_collector_id: None,
        active_topics: topics.clone(),
        buffer_size_bytes: 0,
        total_recorded_bytes: 0,
    };

    let json = serde_json::to_string(&response).unwrap();
    let parsed: StatusResponse = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.active_topics.len(), 100);
    assert_eq!(parsed.active_topics, topics);
}

#[test]
fn test_request_with_special_characters_in_fields() {
    let request = RecorderRequest {
        command: RecorderCommand::Start,
        recording_id: Some("rec-001-special_@#$".to_string()),
        topics: vec!["topic/with/slashes".to_string()],
        scene: Some("scene with spaces".to_string()),
        skills: vec!["skill-1".to_string(), "skill_2".to_string()],
        organization: Some("Org & Co.".to_string()),
        task_id: Some("task#123".to_string()),
        device_id: "device-456".to_string(),
        data_collector_id: Some("collector@789".to_string()),
        compression_type: CompressionType::default(),
        compression_level: CompressionLevel::default(),
    };

    let json = serde_json::to_string(&request).unwrap();
    let parsed: RecorderRequest = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.recording_id, Some("rec-001-special_@#$".to_string()));
    assert_eq!(parsed.organization, Some("Org & Co.".to_string()));
}

#[test]
fn test_error_response_with_empty_message() {
    let response = RecorderResponse::error("".to_string());

    assert!(!response.success);
    assert_eq!(response.message, "");
}

#[test]
fn test_error_response_with_long_message() {
    let long_message = "Error: ".to_string() + &"x".repeat(1000);
    let response = RecorderResponse::error(long_message.clone());

    assert!(!response.success);
    assert_eq!(response.message, long_message);
}

#[test]
fn test_status_key_with_uuid_recording_id() {
    let key = "recorder/status/550e8400-e29b-41d4-a716-446655440000";
    let parts: Vec<&str> = key.split('/').collect();

    assert_eq!(parts.len(), 3);
    assert_eq!(parts[2], "550e8400-e29b-41d4-a716-446655440000");
}

#[test]
fn test_control_key_with_complex_device_id() {
    let key = "recorder/control/device-region-us-west-001";
    let parts: Vec<&str> = key.split('/').collect();

    assert_eq!(parts.len(), 3);
    assert_eq!(parts[2], "device-region-us-west-001");
}

#[test]
fn test_status_response_finished_state() {
    let response = StatusResponse {
        success: true,
        message: "Recording finished".to_string(),
        status: RecordingStatus::Finished,
        scene: Some("scene1".to_string()),
        skills: vec!["skill1".to_string()],
        organization: Some("org".to_string()),
        task_id: Some("task-1".to_string()),
        device_id: "device-1".to_string(),
        data_collector_id: Some("collector-1".to_string()),
        active_topics: vec![],
        buffer_size_bytes: 0,
        total_recorded_bytes: 50000,
    };

    let json = serde_json::to_string(&response).unwrap();
    let parsed: StatusResponse = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.status, RecordingStatus::Finished);
    assert!(parsed.total_recorded_bytes > 0);
    assert_eq!(parsed.buffer_size_bytes, 0);
}

#[test]
fn test_status_response_cancelled_state() {
    let response = StatusResponse {
        success: true,
        message: "Recording cancelled".to_string(),
        status: RecordingStatus::Cancelled,
        scene: None,
        skills: vec![],
        organization: None,
        task_id: None,
        device_id: "device-1".to_string(),
        data_collector_id: None,
        active_topics: vec![],
        buffer_size_bytes: 0,
        total_recorded_bytes: 0,
    };

    let json = serde_json::to_string(&response).unwrap();
    let parsed: StatusResponse = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.status, RecordingStatus::Cancelled);
}

