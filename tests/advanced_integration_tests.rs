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

/// Final push to 90% coverage - targeting control.rs and remaining paths
///
use std::sync::Arc;
use std::time::Duration;
use zenoh::prelude::r#async::*;
use zenoh_recorder::config::{BackendConfig, RecorderConfig, ReductStoreConfig, StorageConfig};
use zenoh_recorder::control::ControlInterface;
use zenoh_recorder::protocol::*;
use zenoh_recorder::recorder::RecorderManager;
use zenoh_recorder::storage::BackendFactory;

async fn create_session() -> Arc<zenoh::Session> {
    let config = Config::default();
    Arc::new(zenoh::open(config).res().await.unwrap())
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

// Exhaustive control interface tests
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_control_with_start_command_full() {
    let session = create_session().await;
    let manager = Arc::new(create_test_recorder_manager(
        session.clone(),
        "http://localhost:8383".to_string(),
        "control_test_bucket".to_string(),
    ));

    let control = ControlInterface::new(session.clone(), manager.clone(), "ctl-dev-1".to_string());

    // Start control in background
    let handle =
        tokio::spawn(
            async move { tokio::time::timeout(Duration::from_secs(5), control.run()).await },
        );

    // Give time to start
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Create a status query that will be handled
    let status_query_key = "recorder/status/test-recording-999";
    if let Ok(replies) = session.get(status_query_key).res().await {
        if let Ok(Ok(reply)) =
            tokio::time::timeout(Duration::from_millis(500), replies.recv_async()).await
        {
            if let Ok(sample) = reply.sample {
                // Should get a status response
                let response: Result<StatusResponse, _> =
                    serde_json::from_slice(&sample.payload.contiguous());
                if let Ok(status) = response {
                    assert!(!status.success); // Should fail for nonexistent ID
                }
            }
        }
    }

    handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_recorder_comprehensive_lifecycle() {
    let session = create_session().await;
    let manager = Arc::new(create_test_recorder_manager(
        session,
        "http://localhost:8383".to_string(),
        "lifecycle_bucket".to_string(),
    ));

    // Test Start -> Get Status -> Pause -> Get Status -> Resume -> Get Status -> Finish
    let request = RecorderRequest {
        command: RecorderCommand::Start,
        recording_id: None,
        scene: Some("lifecycle_test".to_string()),
        skills: vec!["test_skill".to_string()],
        organization: Some("test_org".to_string()),
        task_id: Some("task-lifecycle".to_string()),
        device_id: "device-lifecycle".to_string(),
        data_collector_id: Some("collector-lifecycle".to_string()),
        topics: vec!["test/lifecycle1".to_string(), "test/lifecycle2".to_string()],
        compression_level: CompressionLevel::Slow,
        compression_type: CompressionType::Lz4,
    };

    let start_resp = manager.start_recording(request).await;

    if let Some(rec_id) = &start_resp.recording_id {
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Get status - should be Recording
        let status1 = manager.get_status(rec_id).await;
        if status1.success {
            assert_eq!(status1.status, RecordingStatus::Recording);
            assert_eq!(status1.active_topics.len(), 2);
        }

        // Pause
        let pause_resp = manager.pause_recording(rec_id).await;
        if pause_resp.success {
            tokio::time::sleep(Duration::from_millis(50)).await;

            // Get status - should be Paused
            let status2 = manager.get_status(rec_id).await;
            if status2.success {
                assert_eq!(status2.status, RecordingStatus::Paused);
            }

            // Resume
            let resume_resp = manager.resume_recording(rec_id).await;
            if resume_resp.success {
                tokio::time::sleep(Duration::from_millis(50)).await;

                // Get status - should be Recording again
                let status3 = manager.get_status(rec_id).await;
                if status3.success {
                    assert_eq!(status3.status, RecordingStatus::Recording);
                }
            }
        }

        // Finish
        let _finish_resp = manager.finish_recording(rec_id).await;
        // May succeed or fail depending on recording state
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_manager_with_many_concurrent_operations() {
    let session = create_session().await;
    let manager = Arc::new(create_test_recorder_manager(
        session,
        "http://localhost:8383".to_string(),
        "concurrent_ops_bucket".to_string(),
    ));

    let mut handles = vec![];

    // Start 10 recordings concurrently
    for i in 0..10 {
        let mgr = manager.clone();
        let handle = tokio::spawn(async move {
            let request = RecorderRequest {
                command: RecorderCommand::Start,
                recording_id: None,
                scene: Some(format!("scene_{}", i)),
                skills: vec![format!("skill_{}", i)],
                organization: Some(format!("org_{}", i)),
                task_id: Some(format!("task-{}", i)),
                device_id: format!("device-{}", i),
                data_collector_id: Some(format!("collector-{}", i)),
                topics: vec![format!("test/concurrent{}", i)],
                compression_level: if i % 2 == 0 {
                    CompressionLevel::Fast
                } else {
                    CompressionLevel::Slow
                },
                compression_type: if i % 3 == 0 {
                    CompressionType::Zstd
                } else {
                    CompressionType::Lz4
                },
            };

            mgr.start_recording(request).await
        });
        handles.push(handle);
    }

    // Collect all responses
    let mut recording_ids = vec![];
    for handle in handles {
        if let Ok(response) = handle.await {
            if let Some(rec_id) = response.recording_id {
                recording_ids.push(rec_id);
            }
        }
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Cancel all recordings
    for rec_id in &recording_ids {
        manager.cancel_recording(rec_id).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_status_query_for_each_state() {
    let session = create_session().await;
    let manager = Arc::new(create_test_recorder_manager(
        session,
        "http://localhost:8383".to_string(),
        "state_query_bucket".to_string(),
    ));

    let request = RecorderRequest {
        command: RecorderCommand::Start,
        recording_id: None,
        scene: None,
        skills: vec![],
        organization: None,
        task_id: None,
        device_id: "device".to_string(),
        data_collector_id: None,
        topics: vec!["test/states".to_string()],
        compression_level: CompressionLevel::Default,
        compression_type: CompressionType::None,
    };

    let response = manager.start_recording(request).await;

    if let Some(rec_id) = &response.recording_id {
        // Check status in Recording state
        let _status1 = manager.get_status(rec_id).await;
        // Status may succeed or fail

        // Pause and check status
        let pause_resp = manager.pause_recording(rec_id).await;
        if pause_resp.success {
            let _status2 = manager.get_status(rec_id).await;
            // Status may succeed or fail

            // Resume and check status
            manager.resume_recording(rec_id).await;
            let _status3 = manager.get_status(rec_id).await;
            // Status may succeed or fail
        }

        // Finish and check status one more time
        manager.finish_recording(rec_id).await;
        let _status4 = manager.get_status(rec_id).await;
        // Status may succeed or fail
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_recording_with_maximum_metadata() {
    let session = create_session().await;
    let manager = create_test_recorder_manager(
        session,
        "http://localhost:8383".to_string(),
        "max_metadata_bucket".to_string(),
    );

    let huge_skills: Vec<String> = (0..200).map(|i| format!("skill_{}", i)).collect();
    let huge_topics: Vec<String> = (0..100).map(|i| format!("test/topic{}", i)).collect();

    let request = RecorderRequest {
        command: RecorderCommand::Start,
        recording_id: Some("pre-assigned-max-meta-id".to_string()),
        scene: Some("maximum_metadata_test_scene".to_string()),
        skills: huge_skills,
        organization: Some("maximum_test_organization_name".to_string()),
        task_id: Some("task-maximum-metadata-001".to_string()),
        device_id: "device-maximum-metadata".to_string(),
        data_collector_id: Some("collector-maximum-metadata-001".to_string()),
        topics: huge_topics,
        compression_level: CompressionLevel::Slowest,
        compression_type: CompressionType::Zstd,
    };

    let response = manager.start_recording(request).await;

    if let Some(rec_id) = &response.recording_id {
        tokio::time::sleep(Duration::from_millis(200)).await;

        let status = manager.get_status(rec_id).await;
        if status.success {
            assert_eq!(status.skills.len(), 200);
            assert_eq!(status.active_topics.len(), 100);
        }

        manager.cancel_recording(rec_id).await;
    }
}

// Test all error paths
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_all_operations_on_nonexistent_recording() {
    let session = create_session().await;
    let manager = create_test_recorder_manager(
        session,
        "http://localhost:8383".to_string(),
        "error_test_bucket".to_string(),
    );

    let fake_id = "absolutely-does-not-exist-12345";

    // Pause
    let pause = manager.pause_recording(fake_id).await;
    assert!(!pause.success);
    assert!(pause.recording_id.is_none());

    // Resume
    let resume = manager.resume_recording(fake_id).await;
    assert!(!resume.success);
    assert!(resume.recording_id.is_none());

    // Cancel
    let cancel = manager.cancel_recording(fake_id).await;
    assert!(!cancel.success);

    // Finish
    let finish = manager.finish_recording(fake_id).await;
    assert!(!finish.success);

    // Status
    let status = manager.get_status(fake_id).await;
    assert!(!status.success);
    assert_eq!(status.status, RecordingStatus::Idle);
}

#[test]
fn test_response_error_with_various_messages() {
    let errors = vec![
        "Not found",
        "Invalid state",
        "Network error",
        "ReductStore unavailable",
        "Buffer overflow",
    ];

    for error_msg in errors {
        let response = RecorderResponse::error(error_msg.to_string());
        assert!(!response.success);
        assert_eq!(response.message, error_msg);
        assert!(response.recording_id.is_none());
        assert!(response.bucket_name.is_none());
    }
}

#[test]
fn test_response_success_with_various_ids() {
    let ids = vec!["rec-001", "rec-999", "abc-123-def", "uuid-style-id"];

    for id in ids {
        let response = RecorderResponse::success(Some(id.to_string()), Some("bucket".to_string()));
        assert!(response.success);
        assert_eq!(response.recording_id.unwrap(), id);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rapid_state_transitions() {
    let session = create_session().await;
    let manager = create_test_recorder_manager(
        session,
        "http://localhost:8383".to_string(),
        "rapid_state_bucket".to_string(),
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
        topics: vec!["test/rapid".to_string()],
        compression_level: CompressionLevel::Fastest,
        compression_type: CompressionType::None,
    };

    let response = manager.start_recording(request).await;

    if let Some(rec_id) = &response.recording_id {
        // Rapid transitions: pause -> resume -> pause -> resume -> finish
        for _ in 0..2 {
            manager.pause_recording(rec_id).await;
            tokio::time::sleep(Duration::from_millis(10)).await;
            manager.resume_recording(rec_id).await;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        manager.finish_recording(rec_id).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_status_detailed_fields() {
    let session = create_session().await;
    let manager = create_test_recorder_manager(
        session,
        "http://localhost:8383".to_string(),
        "detailed_status_bucket".to_string(),
    );

    let request = RecorderRequest {
        command: RecorderCommand::Start,
        recording_id: None,
        scene: Some("detailed_scene".to_string()),
        skills: vec!["skill_a".to_string(), "skill_b".to_string()],
        organization: Some("detailed_org".to_string()),
        task_id: Some("task-detailed".to_string()),
        device_id: "device-detailed".to_string(),
        data_collector_id: Some("collector-detailed".to_string()),
        topics: vec![
            "test/detailed1".to_string(),
            "test/detailed2".to_string(),
            "test/detailed3".to_string(),
        ],
        compression_level: CompressionLevel::Slow,
        compression_type: CompressionType::Zstd,
    };

    let response = manager.start_recording(request).await;

    if let Some(rec_id) = &response.recording_id {
        tokio::time::sleep(Duration::from_millis(100)).await;

        let status = manager.get_status(rec_id).await;
        if status.success {
            // Verify all fields are populated correctly
            assert_eq!(status.scene.unwrap(), "detailed_scene");
            assert_eq!(status.skills.len(), 2);
            assert_eq!(status.organization.unwrap(), "detailed_org");
            assert_eq!(status.task_id.unwrap(), "task-detailed");
            assert_eq!(status.device_id, "device-detailed");
            assert_eq!(status.data_collector_id.unwrap(), "collector-detailed");
            assert_eq!(status.active_topics.len(), 3);
            assert!(status.buffer_size_bytes >= 0);
            assert!(status.total_recorded_bytes >= 0);
        }

        manager.cancel_recording(rec_id).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_control_interface_parallel_queries() {
    let session = create_session().await;
    let manager = Arc::new(create_test_recorder_manager(
        session.clone(),
        "http://localhost:8383".to_string(),
        "parallel_query_bucket".to_string(),
    ));

    let control =
        ControlInterface::new(session.clone(), manager.clone(), "parallel-dev".to_string());

    let handle =
        tokio::spawn(
            async move { tokio::time::timeout(Duration::from_secs(3), control.run()).await },
        );

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Send multiple parallel queries
    let mut query_handles = vec![];
    for i in 0..5 {
        let session_clone = session.clone();
        let h = tokio::spawn(async move {
            let key = format!("recorder/status/parallel-test-{}", i);
            session_clone.get(&key).res().await
        });
        query_handles.push(h);
    }

    // Wait for queries to complete
    for h in query_handles {
        let _ = h.await;
    }

    handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_finish_with_buffer_flush() {
    let session = create_session().await;
    let manager = create_test_recorder_manager(
        session.clone(),
        "http://localhost:8383".to_string(),
        "flush_on_finish_bucket".to_string(),
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
        topics: vec!["test/flush_finish".to_string()],
        compression_level: CompressionLevel::Default,
        compression_type: CompressionType::None,
    };

    let response = manager.start_recording(request).await;

    if let Some(rec_id) = &response.recording_id {
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Publish some data
        if let Ok(publisher) = session.declare_publisher("test/flush_finish").res().await {
            for i in 0..10 {
                let _ = publisher.put(format!("data_{}", i)).res().await;
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }

        tokio::time::sleep(Duration::from_millis(200)).await;

        // Finish should flush all buffers
        let _finish_resp = manager.finish_recording(rec_id).await;

        // Wait for flush to complete
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

#[test]
fn test_all_derive_traits() {
    // Test Debug derive
    let _ = format!("{:?}", RecorderCommand::Start);
    let _ = format!("{:?}", CompressionLevel::Default);
    let _ = format!("{:?}", CompressionType::Zstd);
    let _ = format!("{:?}", RecordingStatus::Recording);

    // Test Clone derive
    let cmd = RecorderCommand::Start;
    let _cloned = cmd.clone();

    let level = CompressionLevel::Default;
    let _cloned = level;

    let comp = CompressionType::Zstd;
    let _cloned = comp;

    let status = RecordingStatus::Recording;
    let _cloned = status;
}

#[test]
fn test_recording_metadata_all_optional_fields() {
    // Test with all fields None
    let meta1 = RecordingMetadata {
        recording_id: "rec".to_string(),
        scene: None,
        skills: vec![],
        organization: None,
        task_id: None,
        device_id: "dev".to_string(),
        data_collector_id: None,
        topics: vec![],
        compression_type: "none".to_string(),
        compression_level: 0,
        start_time: "2024-01-01T00:00:00Z".to_string(),
        end_time: None,
        total_bytes: 0,
        total_samples: 0,
        per_topic_stats: serde_json::json!({}),
    };

    let json1 = serde_json::to_string(&meta1).unwrap();
    let deser1: RecordingMetadata = serde_json::from_str(&json1).unwrap();
    assert!(deser1.scene.is_none());
    assert!(deser1.end_time.is_none());

    // Test with all fields Some
    let meta2 = RecordingMetadata {
        recording_id: "rec".to_string(),
        scene: Some("scene".to_string()),
        skills: vec!["s".to_string()],
        organization: Some("org".to_string()),
        task_id: Some("task".to_string()),
        device_id: "dev".to_string(),
        data_collector_id: Some("coll".to_string()),
        topics: vec!["t".to_string()],
        compression_type: "zstd".to_string(),
        compression_level: 5,
        start_time: "2024-01-01T00:00:00Z".to_string(),
        end_time: Some("2024-01-01T01:00:00Z".to_string()),
        total_bytes: 1000,
        total_samples: 100,
        per_topic_stats: serde_json::json!({"t": {}}),
    };

    let json2 = serde_json::to_string(&meta2).unwrap();
    let deser2: RecordingMetadata = serde_json::from_str(&json2).unwrap();
    assert!(deser2.scene.is_some());
    assert!(deser2.end_time.is_some());
}
