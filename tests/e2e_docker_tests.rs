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

// End-to-end tests with Docker Zenoh + ReductStore
// Requires both services running on ports 27447 and 28383

use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use zenoh::prelude::r#async::*;
use zenoh_recorder::config::{BackendConfig, RecorderConfig, ReductStoreConfig, StorageConfig};
use zenoh_recorder::control::ControlInterface;
use zenoh_recorder::protocol::{
    CompressionLevel, CompressionType, RecorderCommand, RecorderRequest, RecordingStatus,
    StatusResponse,
};
use zenoh_recorder::recorder::RecorderManager;
use zenoh_recorder::storage::BackendFactory;

// Helper to get Zenoh endpoint
fn get_zenoh_endpoint() -> String {
    env::var("ZENOH_TEST_ENDPOINT").unwrap_or_else(|_| "tcp/127.0.0.1:27447".to_string())
}

// Helper to get ReductStore URL
fn get_reductstore_url() -> String {
    env::var("REDUCTSTORE_TEST_URL").unwrap_or_else(|_| "http://127.0.0.1:28383".to_string())
}

// Helper to get test bucket name
fn get_test_bucket() -> String {
    env::var("REDUCTSTORE_TEST_BUCKET").unwrap_or_else(|_| "zenoh-e2e-test".to_string())
}

// Helper to check if services are available
async fn are_services_available() -> bool {
    // Check ReductStore
    let reductstore_url = get_reductstore_url();
    let client = reqwest::Client::new();
    let info_url = format!("{}/api/v1/info", reductstore_url);

    let reductstore_ok = match client.get(&info_url).send().await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    };

    // Check Zenoh (test connection)
    let zenoh_ok = zenoh::open(config::peer()).res().await.is_ok();

    reductstore_ok && zenoh_ok
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_e2e_control_interface_query() {
    if !are_services_available().await {
        eprintln!("Skipping E2E test: Services not available");
        return;
    }

    // Create Zenoh session
    let mut config = config::peer();
    config.listen.endpoints = vec![get_zenoh_endpoint().parse().unwrap()];
    config.connect.endpoints = vec![get_zenoh_endpoint().parse().unwrap()];

    let session = zenoh::open(config).res().await.unwrap();
    let session_arc = Arc::new(session);

    // Create recorder manager
    let manager = Arc::new(create_test_recorder_manager(
        session_arc.clone(),
        get_reductstore_url(),
        get_test_bucket(),
    ));

    // Start control interface in background
    let control = ControlInterface::new(
        session_arc.clone(),
        manager.clone(),
        "test-device-001".to_string(),
    );

    let control_handle = tokio::spawn(async move {
        // Run for a short time
        tokio::time::timeout(Duration::from_secs(5), control.run()).await
    });

    // Give it time to set up queryables
    sleep(Duration::from_millis(500)).await;

    // Test status query using Zenoh
    let status_key = "recorder/status/test-recording";
    let replies = session_arc.get(status_key).res().await.unwrap();

    // Should get a response
    let mut got_response = false;
    while let Ok(reply) = replies.recv_async().await {
        match reply.sample {
            Ok(sample) => {
                let bytes = sample.payload.contiguous();
                let response: StatusResponse = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(response.status, RecordingStatus::Idle);
                got_response = true;
                break;
            }
            Err(e) => {
                eprintln!("Error in reply: {:?}", e);
            }
        }
    }

    assert!(got_response, "Should receive status response");

    // Cancel the control interface
    control_handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_e2e_recorder_manager_with_reductstore() {
    if !are_services_available().await {
        eprintln!("Skipping E2E test: Services not available");
        return;
    }

    // Create Zenoh session
    let session = zenoh::open(config::peer()).res().await.unwrap();
    let session_arc = Arc::new(session);

    // Create recorder manager
    let manager =
        create_test_recorder_manager(session_arc, get_reductstore_url(), get_test_bucket());

    // Create a start recording request (recording_id is None - server generates it)
    let request = RecorderRequest {
        command: RecorderCommand::Start,
        recording_id: None, // Server generates the ID
        topics: vec!["test/topic1".to_string(), "test/topic2".to_string()],
        scene: Some("e2e_test_scene".to_string()),
        skills: vec!["skill1".to_string()],
        organization: Some("test_org".to_string()),
        task_id: Some("task-001".to_string()),
        device_id: "device-001".to_string(),
        data_collector_id: Some("collector-001".to_string()),
        compression_type: CompressionType::Zstd,
        compression_level: CompressionLevel::Default,
    };

    // Start recording
    let response = manager.start_recording(request).await;
    assert!(response.success, "Recording should start successfully");
    assert!(response.recording_id.is_some());

    // Get the generated recording_id
    let recording_id = response.recording_id.unwrap();

    // Get status
    let status = manager.get_status(&recording_id).await;
    assert!(status.success);
    assert_eq!(status.status, RecordingStatus::Recording);

    // Pause recording
    let pause_response = manager.pause_recording(&recording_id).await;
    assert!(pause_response.success);

    // Check paused status
    let status = manager.get_status(&recording_id).await;
    assert_eq!(status.status, RecordingStatus::Paused);

    // Resume recording
    let resume_response = manager.resume_recording(&recording_id).await;
    assert!(resume_response.success);

    // Check resumed status
    let status = manager.get_status(&recording_id).await;
    assert_eq!(status.status, RecordingStatus::Recording);

    // Finish recording
    let finish_response = manager.finish_recording(&recording_id).await;
    assert!(finish_response.success);

    // Check finished status
    let status = manager.get_status(&recording_id).await;
    assert_eq!(status.status, RecordingStatus::Finished);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_e2e_multiple_recordings() {
    if !are_services_available().await {
        eprintln!("Skipping E2E test: Services not available");
        return;
    }

    let session = zenoh::open(config::peer()).res().await.unwrap();
    let session_arc = Arc::new(session);

    let manager =
        create_test_recorder_manager(session_arc, get_reductstore_url(), get_test_bucket());

    // Start multiple recordings and collect their IDs
    let mut recording_ids = Vec::new();

    for i in 1..=3 {
        let request = RecorderRequest {
            command: RecorderCommand::Start,
            recording_id: None, // Server generates
            topics: vec![format!("test/topic/multi{}", i)],
            scene: Some("multi_test".to_string()),
            skills: vec![],
            organization: None,
            task_id: None,
            device_id: "device-001".to_string(),
            data_collector_id: None,
            compression_type: CompressionType::Zstd,
            compression_level: CompressionLevel::Default,
        };

        let response = manager.start_recording(request).await;
        assert!(response.success);
        recording_ids.push(response.recording_id.unwrap());
    }

    // Verify all are recording
    for recording_id in &recording_ids {
        let status = manager.get_status(recording_id).await;
        assert_eq!(status.status, RecordingStatus::Recording);
    }

    // Finish all recordings
    for recording_id in &recording_ids {
        let response = manager.finish_recording(recording_id).await;
        assert!(response.success);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_e2e_recording_with_compression_types() {
    if !are_services_available().await {
        eprintln!("Skipping E2E test: Services not available");
        return;
    }

    let session = zenoh::open(config::peer()).res().await.unwrap();
    let session_arc = Arc::new(session);

    let manager =
        create_test_recorder_manager(session_arc, get_reductstore_url(), get_test_bucket());

    let compression_types = vec![
        CompressionType::None,
        CompressionType::Lz4,
        CompressionType::Zstd,
    ];

    for compression_type in compression_types.into_iter() {
        let request = RecorderRequest {
            command: RecorderCommand::Start,
            recording_id: None, // Server generates
            topics: vec!["test/compression".to_string()],
            scene: None,
            skills: vec![],
            organization: None,
            task_id: None,
            device_id: "device-001".to_string(),
            data_collector_id: None,
            compression_type,
            compression_level: CompressionLevel::Default,
        };

        let response = manager.start_recording(request).await;
        assert!(
            response.success,
            "Failed with compression {:?}",
            compression_type
        );

        let recording_id = response.recording_id.unwrap();

        // Finish immediately
        let finish_response = manager.finish_recording(&recording_id).await;
        assert!(finish_response.success);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_e2e_cancel_recording() {
    if !are_services_available().await {
        eprintln!("Skipping E2E test: Services not available");
        return;
    }

    let session = zenoh::open(config::peer()).res().await.unwrap();
    let session_arc = Arc::new(session);

    let manager =
        create_test_recorder_manager(session_arc, get_reductstore_url(), get_test_bucket());

    let request = RecorderRequest {
        command: RecorderCommand::Start,
        recording_id: None, // Server generates
        topics: vec!["test/cancel".to_string()],
        scene: None,
        skills: vec![],
        organization: None,
        task_id: None,
        device_id: "device-001".to_string(),
        data_collector_id: None,
        compression_type: CompressionType::Zstd,
        compression_level: CompressionLevel::Default,
    };

    // Start recording
    let response = manager.start_recording(request).await;
    assert!(response.success);
    let recording_id = response.recording_id.unwrap();

    // Cancel recording
    let cancel_response = manager.cancel_recording(&recording_id).await;
    assert!(cancel_response.success);

    // Check cancelled status
    let status = manager.get_status(&recording_id).await;
    assert_eq!(status.status, RecordingStatus::Cancelled);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_e2e_error_handling() {
    if !are_services_available().await {
        eprintln!("Skipping E2E test: Services not available");
        return;
    }

    let session = zenoh::open(config::peer()).res().await.unwrap();
    let session_arc = Arc::new(session);

    let manager =
        create_test_recorder_manager(session_arc, get_reductstore_url(), get_test_bucket());

    // Try to pause non-existent recording
    let response = manager.pause_recording("nonexistent").await;
    assert!(!response.success);
    assert!(response.message.contains("not found") || response.message.contains("does not exist"));

    // Try to resume non-existent recording
    let response = manager.resume_recording("nonexistent").await;
    assert!(!response.success);

    // Try to finish non-existent recording
    let response = manager.finish_recording("nonexistent").await;
    assert!(!response.success);

    // Try to cancel non-existent recording
    let response = manager.cancel_recording("nonexistent").await;
    assert!(!response.success);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_e2e_recording_lifecycle_with_metadata() {
    if !are_services_available().await {
        eprintln!("Skipping E2E test: Services not available");
        return;
    }

    let session = zenoh::open(config::peer()).res().await.unwrap();
    let session_arc = Arc::new(session);

    let manager =
        create_test_recorder_manager(session_arc, get_reductstore_url(), get_test_bucket());

    let request = RecorderRequest {
        command: RecorderCommand::Start,
        recording_id: None, // Server generates
        topics: vec![
            "test/sensor/lidar".to_string(),
            "test/sensor/camera".to_string(),
            "test/control/cmd".to_string(),
        ],
        scene: Some("warehouse_navigation".to_string()),
        skills: vec!["navigation".to_string(), "object_detection".to_string()],
        organization: Some("test_company".to_string()),
        task_id: Some("task-12345".to_string()),
        device_id: "robot-001".to_string(),
        data_collector_id: Some("collector-001".to_string()),
        compression_type: CompressionType::Zstd,
        compression_level: CompressionLevel::Slow,
    };

    // Start recording
    let response = manager.start_recording(request).await;
    assert!(response.success);
    let recording_id = response.recording_id.unwrap();

    // Verify metadata in status
    let status = manager.get_status(&recording_id).await;
    assert_eq!(status.scene, Some("warehouse_navigation".to_string()));
    assert_eq!(status.skills.len(), 2);
    assert_eq!(status.organization, Some("test_company".to_string()));
    assert_eq!(status.task_id, Some("task-12345".to_string()));
    assert_eq!(status.device_id, "robot-001");
    assert_eq!(status.data_collector_id, Some("collector-001".to_string()));
    assert_eq!(status.active_topics.len(), 3);

    // Finish recording
    let finish_response = manager.finish_recording(&recording_id).await;
    assert!(finish_response.success);
}
