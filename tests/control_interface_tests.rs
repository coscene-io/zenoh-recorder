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

/// Control interface tests
///
/// Tests for the Zenoh Queryable-based control interface
/// These tests validate request handling, error cases, and protocol compliance
///
use std::sync::Arc;
use std::time::Duration;
use zenoh::prelude::r#async::*;
use zenoh_recorder::control::ControlInterface;
use zenoh_recorder::protocol::*;
use zenoh_recorder::recorder::RecorderManager;

/// Helper to create a test session
async fn create_test_session() -> Result<Arc<zenoh::Session>, String> {
    let config = Config::default();
    zenoh::open(config)
        .res()
        .await
        .map(Arc::new)
        .map_err(|e| format!("{}", e))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_control_interface_creation() {
    let session = create_test_session().await.unwrap();
    let manager = Arc::new(RecorderManager::new(
        session.clone(),
        "http://localhost:8383".to_string(),
        "test_bucket".to_string(),
    ));

    let control = ControlInterface::new(session.clone(), manager, "test-device".to_string());
    
    // Just verify it can be created
    drop(control);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_control_interface_run_timeout() {
    let session = create_test_session().await.unwrap();
    let manager = Arc::new(RecorderManager::new(
        session.clone(),
        "http://localhost:8383".to_string(),
        "test_bucket".to_string(),
    ));

    let control = ControlInterface::new(session.clone(), manager, "test-device-2".to_string());
    
    // Run with timeout to avoid blocking forever
    let result = tokio::time::timeout(Duration::from_millis(500), control.run()).await;
    
    // Should timeout since there are no queries
    assert!(result.is_err(), "Control interface should timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_control_interface_with_query() {
    let session = create_test_session().await.unwrap();
    let manager = Arc::new(RecorderManager::new(
        session.clone(),
        "http://localhost:8383".to_string(),
        "test_bucket".to_string(),
    ));

    let device_id = "test-device-query";
    let control = ControlInterface::new(session.clone(), manager.clone(), device_id.to_string());
    
    // Spawn control interface in background
    let control_handle = tokio::spawn(async move {
        tokio::time::timeout(Duration::from_secs(2), control.run()).await
    });

    // Give it time to start
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Try to query status (should get error for nonexistent recording)
    let status_key = "recorder/status/nonexistent-123";
    if let Ok(replies) = session.get(status_key).res().await.map_err(|e| format!("{}", e)) {
        tokio::time::timeout(Duration::from_millis(500), async {
            while let Ok(reply) = replies.recv_async().await {
                match reply.sample {
                    Ok(sample) => {
                        let response: Result<StatusResponse, _> =
                            serde_json::from_slice(&sample.payload.contiguous());
                        if let Ok(resp) = response {
                            assert!(!resp.success);
                            break;
                        }
                    }
                    Err(_) => continue,
                }
            }
        })
        .await
        .ok();
    }

    // Clean up
    control_handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_multiple_control_interfaces() {
    let session = create_test_session().await.unwrap();
    
    // Create multiple control interfaces for different devices
    let devices = vec!["device-1", "device-2", "device-3"];
    let mut interfaces = Vec::new();

    for device in devices {
        let manager = Arc::new(RecorderManager::new(
            session.clone(),
            "http://localhost:8383".to_string(),
            format!("bucket_{}", device),
        ));

        let control = ControlInterface::new(
            session.clone(),
            manager,
            device.to_string(),
        );

        interfaces.push(control);
    }

    assert_eq!(interfaces.len(), 3);
}

#[test]
fn test_recorder_request_all_commands() {
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
            recording_id: Some("test-123".to_string()),
            scene: None,
            skills: vec![],
            organization: None,
            task_id: None,
            device_id: "device-01".to_string(),
            data_collector_id: None,
            topics: vec![],
            compression_level: CompressionLevel::Default,
            compression_type: CompressionType::Zstd,
        };

        // Verify serialization works for all commands
        let json = serde_json::to_string(&request).unwrap();
        assert!(!json.is_empty());

        let deserialized: RecorderRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.device_id, "device-01");
    }
}

#[test]
fn test_status_response_all_states() {
    let states = vec![
        RecordingStatus::Idle,
        RecordingStatus::Recording,
        RecordingStatus::Paused,
        RecordingStatus::Uploading,
        RecordingStatus::Finished,
        RecordingStatus::Cancelled,
    ];

    for state in states {
        let response = StatusResponse {
            success: true,
            message: "test".to_string(),
            status: state,
            scene: None,
            skills: vec![],
            organization: None,
            task_id: None,
            device_id: "dev".to_string(),
            data_collector_id: None,
            active_topics: vec![],
            buffer_size_bytes: 0,
            total_recorded_bytes: 0,
        };

        // Verify serialization works for all states
        let json = serde_json::to_string(&response).unwrap();
        let deserialized: StatusResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.status, state);
    }
}

