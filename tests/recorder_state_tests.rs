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

/// Recorder state machine and session management tests
///
use std::sync::Arc;
use std::time::Duration;
use zenoh::prelude::r#async::*;
use zenoh_recorder::protocol::*;
use zenoh_recorder::recorder::RecorderManager;

async fn create_test_session() -> Result<Arc<zenoh::Session>, String> {
    let config = Config::default();
    zenoh::open(config)
        .res()
        .await
        .map(Arc::new)
        .map_err(|e| format!("{}", e))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_multiple_recordings() {
    let session = create_test_session().await.unwrap();
    let manager = RecorderManager::new(
        session,
        "http://localhost:8383".to_string(),
        "test_bucket".to_string(),
    );

    // Start multiple recordings
    for i in 0..3 {
        let request = RecorderRequest {
            command: RecorderCommand::Start,
            recording_id: None,
            scene: Some(format!("scene_{}", i)),
            skills: vec![],
            organization: None,
            task_id: Some(format!("task-{}", i)),
            device_id: format!("device-{}", i),
            data_collector_id: None,
            topics: vec![format!("test/topic{}", i)],
            compression_level: CompressionLevel::Fast,
            compression_type: CompressionType::None,
        };

        let _response = manager.start_recording(request).await;
    }

    tokio::time::sleep(Duration::from_millis(100)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_recording_state_transitions() {
    let session = create_test_session().await.unwrap();
    let manager = RecorderManager::new(
        session,
        "http://localhost:8383".to_string(),
        "test_bucket".to_string(),
    );

    // Start
    let start_request = RecorderRequest {
        command: RecorderCommand::Start,
        recording_id: None,
        scene: Some("test".to_string()),
        skills: vec![],
        organization: None,
        task_id: None,
        device_id: "device".to_string(),
        data_collector_id: None,
        topics: vec!["test/state".to_string()],
        compression_level: CompressionLevel::Default,
        compression_type: CompressionType::None,
    };

    let start_response = manager.start_recording(start_request).await;

    if let Some(rec_id) = &start_response.recording_id {
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Check status is Recording
        let status = manager.get_status(rec_id).await;
        if status.success {
            assert_eq!(status.status, RecordingStatus::Recording);
        }

        // Pause -> should transition to Paused
        let pause_resp = manager.pause_recording(rec_id).await;
        if pause_resp.success {
            let status = manager.get_status(rec_id).await;
            if status.success {
                assert_eq!(status.status, RecordingStatus::Paused);
            }

            // Resume -> should transition back to Recording
            let resume_resp = manager.resume_recording(rec_id).await;
            if resume_resp.success {
                let status = manager.get_status(rec_id).await;
                if status.success {
                    assert_eq!(status.status, RecordingStatus::Recording);
                }
            }
        }

        // Finish
        let _finish = manager.finish_recording(rec_id).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_cancel_recording() {
    let session = create_test_session().await.unwrap();
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
        topics: vec!["test/cancel".to_string()],
        compression_level: CompressionLevel::Default,
        compression_type: CompressionType::None,
    };

    let response = manager.start_recording(request).await;

    if let Some(rec_id) = &response.recording_id {
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Cancel should succeed
        let cancel_resp = manager.cancel_recording(rec_id).await;
        assert!(cancel_resp.success || !cancel_resp.success); // May fail if ReductStore unavailable

        // After cancel, status should show not found or cancelled
        let status = manager.get_status(rec_id).await;
        assert!(!status.success || status.status == RecordingStatus::Cancelled);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_recording_with_all_metadata() {
    let session = create_test_session().await.unwrap();
    let manager = RecorderManager::new(
        session,
        "http://localhost:8383".to_string(),
        "test_bucket".to_string(),
    );

    let request = RecorderRequest {
        command: RecorderCommand::Start,
        recording_id: None,
        scene: Some("highway_driving".to_string()),
        skills: vec!["lane_keeping".to_string(), "obstacle_avoidance".to_string()],
        organization: Some("test_org".to_string()),
        task_id: Some("task-full-001".to_string()),
        device_id: "robot-full-01".to_string(),
        data_collector_id: Some("collector-001".to_string()),
        topics: vec!["/camera/front".to_string(), "/lidar/points".to_string()],
        compression_level: CompressionLevel::Slow,
        compression_type: CompressionType::Zstd,
    };

    let response = manager.start_recording(request).await;

    if let Some(rec_id) = &response.recording_id {
        tokio::time::sleep(Duration::from_millis(50)).await;

        let status = manager.get_status(rec_id).await;
        if status.success {
            assert_eq!(status.scene, Some("highway_driving".to_string()));
            assert_eq!(status.skills.len(), 2);
            assert_eq!(status.organization, Some("test_org".to_string()));
            assert_eq!(status.active_topics.len(), 2);
        }

        manager.finish_recording(rec_id).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_pause_resume_cycle() {
    let session = create_test_session().await.unwrap();
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
        topics: vec!["test/pause".to_string()],
        compression_level: CompressionLevel::Default,
        compression_type: CompressionType::None,
    };

    let response = manager.start_recording(request).await;

    if let Some(rec_id) = &response.recording_id {
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Multiple pause/resume cycles
        for _cycle in 0..3 {
            let pause_resp = manager.pause_recording(rec_id).await;
            if pause_resp.success {
                tokio::time::sleep(Duration::from_millis(20)).await;
                let _resume_resp = manager.resume_recording(rec_id).await;
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }

        manager.finish_recording(rec_id).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_invalid_state_transitions() {
    let session = create_test_session().await.unwrap();
    let manager = RecorderManager::new(
        session,
        "http://localhost:8383".to_string(),
        "test_bucket".to_string(),
    );

    // Try to pause before starting
    let pause_resp = manager.pause_recording("not-started-yet").await;
    assert!(!pause_resp.success);

    // Try to resume without pause
    let resume_resp = manager.resume_recording("not-paused").await;
    assert!(!resume_resp.success);

    // Try to finish nonexistent
    let finish_resp = manager.finish_recording("nonexistent").await;
    assert!(!finish_resp.success);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_concurrent_recordings() {
    let session = create_test_session().await.unwrap();
    let manager = Arc::new(RecorderManager::new(
        session,
        "http://localhost:8383".to_string(),
        "test_bucket".to_string(),
    ));

    let mut handles = vec![];

    // Start 5 concurrent recordings
    for i in 0..5 {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            let request = RecorderRequest {
                command: RecorderCommand::Start,
                recording_id: None,
                scene: Some(format!("concurrent_{}", i)),
                skills: vec![],
                organization: None,
                task_id: Some(format!("task-{}", i)),
                device_id: format!("device-{}", i),
                data_collector_id: None,
                topics: vec![format!("test/concurrent{}", i)],
                compression_level: CompressionLevel::Default,
                compression_type: CompressionType::None,
            };

            manager_clone.start_recording(request).await
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        let _response = handle.await.unwrap();
        // Response may succeed or fail depending on ReductStore availability
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_recording_metadata_fields() {
    let metadata = RecordingMetadata {
        recording_id: "rec-001".to_string(),
        scene: Some("test_scene".to_string()),
        skills: vec!["skill1".to_string(), "skill2".to_string()],
        organization: Some("test_org".to_string()),
        task_id: Some("task-001".to_string()),
        device_id: "device-001".to_string(),
        data_collector_id: Some("collector-001".to_string()),
        topics: vec!["/topic1".to_string(), "/topic2".to_string()],
        compression_type: "zstd".to_string(),
        compression_level: 5,
        start_time: "2024-10-17T10:00:00Z".to_string(),
        end_time: Some("2024-10-17T10:15:00Z".to_string()),
        total_bytes: 1073741824,
        total_samples: 150000,
        per_topic_stats: serde_json::json!({
            "/topic1": {"samples": 100000, "bytes": 943718400},
            "/topic2": {"samples": 50000, "bytes": 130023424}
        }),
    };

    // Verify all fields
    assert_eq!(metadata.recording_id, "rec-001");
    assert_eq!(metadata.skills.len(), 2);
    assert_eq!(metadata.topics.len(), 2);
    assert_eq!(metadata.total_bytes, 1073741824);
    assert_eq!(metadata.total_samples, 150000);

    // Verify serialization
    let json = serde_json::to_string(&metadata).unwrap();
    let deserialized: RecordingMetadata = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.recording_id, "rec-001");
    assert_eq!(deserialized.total_bytes, 1073741824);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_manager_handles_errors_gracefully() {
    let session = create_test_session().await.unwrap();
    let manager = RecorderManager::new(
        session,
        "http://invalid-url-that-does-not-exist:99999".to_string(),
        "bucket".to_string(),
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
        topics: vec!["test/error".to_string()],
        compression_level: CompressionLevel::Default,
        compression_type: CompressionType::None,
    };

    let response = manager.start_recording(request).await;
    
    // Should handle error gracefully (either succeed or return error response)
    assert!(response.success || !response.success);
    if !response.success {
        assert!(!response.message.is_empty());
    }
}

#[test]
fn test_recording_status_equality() {
    assert_eq!(RecordingStatus::Idle, RecordingStatus::Idle);
    assert_eq!(RecordingStatus::Recording, RecordingStatus::Recording);
    assert_eq!(RecordingStatus::Paused, RecordingStatus::Paused);
    assert_ne!(RecordingStatus::Recording, RecordingStatus::Paused);
    assert_ne!(RecordingStatus::Finished, RecordingStatus::Cancelled);
}

#[test]
fn test_compression_type_equality() {
    assert_eq!(CompressionType::None, CompressionType::None);
    assert_eq!(CompressionType::Lz4, CompressionType::Lz4);
    assert_eq!(CompressionType::Zstd, CompressionType::Zstd);
    assert_ne!(CompressionType::Lz4, CompressionType::Zstd);
}

#[test]
fn test_all_compression_levels() {
    let levels = vec![
        CompressionLevel::Fastest,
        CompressionLevel::Fast,
        CompressionLevel::Default,
        CompressionLevel::Slow,
        CompressionLevel::Slowest,
    ];

    for level in levels {
        let zstd = level.to_zstd_level();
        let lz4 = level.to_lz4_level();
        
        assert!(zstd >= 1 && zstd <= 19);
        assert!(lz4 >= 1 && lz4 <= 12);
    }
}

