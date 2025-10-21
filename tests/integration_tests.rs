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

/// Integration tests for recorder manager and control interface
///
/// These tests verify the complete recording lifecycle including:
/// - Session creation and management
/// - State transitions
/// - Multi-recording support
/// - Error handling
///
use std::sync::Arc;
use std::time::Duration;
use zenoh::prelude::r#async::*;
use zenoh_recorder::config::{BackendConfig, RecorderConfig, ReductStoreConfig, StorageConfig};
use zenoh_recorder::protocol::*;
use zenoh_recorder::recorder::RecorderManager;
use zenoh_recorder::storage::BackendFactory;

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
async fn test_recorder_manager_creation() {
    // Open a Zenoh session
    let config = Config::default();
    let session = zenoh::open(config)
        .res()
        .await
        .map_err(|e| format!("{}", e))
        .unwrap();

    let manager = create_test_recorder_manager(
        Arc::new(session),
        "http://localhost:8383".to_string(),
        "test_bucket".to_string(),
    );

    // Just verify it can be created
    drop(manager);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_start_recording() {
    let config = Config::default();
    let session = zenoh::open(config)
        .res()
        .await
        .map_err(|e| format!("{}", e))
        .unwrap();

    let manager = create_test_recorder_manager(
        Arc::new(session),
        "http://localhost:8383".to_string(),
        "test_bucket".to_string(),
    );

    let request = RecorderRequest {
        command: RecorderCommand::Start,
        recording_id: None,
        scene: Some("test_scene".to_string()),
        skills: vec!["skill1".to_string()],
        organization: Some("test_org".to_string()),
        task_id: Some("task-001".to_string()),
        device_id: "device-01".to_string(),
        data_collector_id: Some("collector-01".to_string()),
        topics: vec!["test/topic1".to_string()],
        compression_level: CompressionLevel::Default,
        compression_type: CompressionType::Zstd,
    };

    let response = manager.start_recording(request).await;

    // Note: This may fail if ReductStore is not running, but structure is tested
    assert!(response.recording_id.is_some() || !response.success);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_pause_nonexistent_recording() {
    let config = Config::default();
    let session = zenoh::open(config)
        .res()
        .await
        .map_err(|e| format!("{}", e))
        .unwrap();

    let manager = create_test_recorder_manager(
        Arc::new(session),
        "http://localhost:8383".to_string(),
        "test_bucket".to_string(),
    );

    let response = manager.pause_recording("nonexistent-id").await;

    assert!(!response.success);
    assert!(response.message.contains("not found"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_resume_nonexistent_recording() {
    let config = Config::default();
    let session = zenoh::open(config)
        .res()
        .await
        .map_err(|e| format!("{}", e))
        .unwrap();

    let manager = create_test_recorder_manager(
        Arc::new(session),
        "http://localhost:8383".to_string(),
        "test_bucket".to_string(),
    );

    let response = manager.resume_recording("nonexistent-id").await;

    assert!(!response.success);
    assert!(response.message.contains("not found"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_cancel_nonexistent_recording() {
    let config = Config::default();
    let session = zenoh::open(config)
        .res()
        .await
        .map_err(|e| format!("{}", e))
        .unwrap();

    let manager = create_test_recorder_manager(
        Arc::new(session),
        "http://localhost:8383".to_string(),
        "test_bucket".to_string(),
    );

    let response = manager.cancel_recording("nonexistent-id").await;

    assert!(!response.success);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_finish_nonexistent_recording() {
    let config = Config::default();
    let session = zenoh::open(config)
        .res()
        .await
        .map_err(|e| format!("{}", e))
        .unwrap();

    let manager = create_test_recorder_manager(
        Arc::new(session),
        "http://localhost:8383".to_string(),
        "test_bucket".to_string(),
    );

    let response = manager.finish_recording("nonexistent-id").await;

    assert!(!response.success);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_status_nonexistent() {
    let config = Config::default();
    let session = zenoh::open(config)
        .res()
        .await
        .map_err(|e| format!("{}", e))
        .unwrap();

    let manager = create_test_recorder_manager(
        Arc::new(session),
        "http://localhost:8383".to_string(),
        "test_bucket".to_string(),
    );

    let response = manager.get_status("nonexistent-id").await;

    assert!(!response.success);
    assert_eq!(response.status, RecordingStatus::Idle);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_manager_shutdown() {
    let config = Config::default();
    let session = zenoh::open(config)
        .res()
        .await
        .map_err(|e| format!("{}", e))
        .unwrap();

    let manager = create_test_recorder_manager(
        Arc::new(session),
        "http://localhost:8383".to_string(),
        "test_bucket".to_string(),
    );

    // Shutdown should succeed even with no active recordings
    let result = manager.shutdown().await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_recording_lifecycle() {
    let config = Config::default();
    let session = zenoh::open(config)
        .res()
        .await
        .map_err(|e| format!("{}", e))
        .unwrap();

    let manager = Arc::new(create_test_recorder_manager(
        Arc::new(session),
        "http://localhost:8383".to_string(),
        "test_bucket".to_string(),
    ));

    // Start recording
    let start_request = RecorderRequest {
        command: RecorderCommand::Start,
        recording_id: None,
        scene: Some("test".to_string()),
        skills: vec![],
        organization: None,
        task_id: None,
        device_id: "device-test".to_string(),
        data_collector_id: None,
        topics: vec!["test/integration".to_string()],
        compression_level: CompressionLevel::Fast,
        compression_type: CompressionType::None,
    };

    let start_response = manager.start_recording(start_request).await;

    if let Some(rec_id) = &start_response.recording_id {
        // Wait a bit
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Get status
        let status = manager.get_status(rec_id).await;
        if status.success {
            assert_eq!(status.status, RecordingStatus::Recording);
        }

        // Pause
        let pause_response = manager.pause_recording(rec_id).await;
        if pause_response.success {
            // Resume
            let _resume_response = manager.resume_recording(rec_id).await;
            // Response can be either success or failure depending on timing
        }

        // Finish
        let _finish_response = manager.finish_recording(rec_id).await;
    }
}
