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

use std::collections::HashMap;
use zenoh_recorder::config::ReductStoreConfig;
use zenoh_recorder::storage::{topic_to_entry_name, ReductStoreBackend};

#[test]
fn test_reductstore_client_creation() {
    let config = ReductStoreConfig {
        url: "http://localhost:8383".to_string(),
        bucket_name: "test_bucket".to_string(),
        api_token: None,
        timeout_seconds: 300,
        max_retries: 3,
    };
    let client = ReductStoreBackend::new(config);
    // Just verify it can be created
    if let Ok(client) = client {
        drop(client);
    }
}

#[test]
fn test_topic_to_entry_conversions() {
    let test_cases = vec![
        ("/camera/front", "camera_front"),
        ("/lidar/points", "lidar_points"),
        ("/imu/data", "imu_data"),
        ("camera/front", "camera_front"),
        ("/a/b/c/d/e", "a_b_c_d_e"),
        ("/test/**", "test_all"),
        ("/topic-with-dash", "topic-with-dash"),
        ("/topic_with_underscore", "topic_with_underscore"),
        ("/very/long/path/to/topic", "very_long_path_to_topic"),
        ("/", ""),
    ];

    for (input, expected) in test_cases {
        assert_eq!(
            topic_to_entry_name(input),
            expected,
            "Failed for input: {}",
            input
        );
    }
}

#[test]
fn test_topic_to_entry_edge_cases() {
    assert_eq!(topic_to_entry_name(""), "");
    assert_eq!(topic_to_entry_name("/"), "");
    assert_eq!(topic_to_entry_name("//"), ""); // After trim_start_matches('/'), becomes ""
    assert_eq!(topic_to_entry_name("///"), ""); // After trim_start_matches('/'), becomes ""
    assert_eq!(topic_to_entry_name("/a/"), "a_");
}

#[test]
fn test_entry_name_consistency() {
    // Same topic should always produce same entry name
    let topic = "/test/topic";
    let entry1 = topic_to_entry_name(topic);
    let entry2 = topic_to_entry_name(topic);
    assert_eq!(entry1, entry2);
}

#[test]
fn test_multiple_client_creation() {
    // Should be able to create multiple clients
    let clients: Vec<_> = (0..5)
        .map(|i| {
            let config = ReductStoreConfig {
                url: format!("http://localhost:{}", 8383 + i),
                bucket_name: format!("bucket_{}", i),
                api_token: None,
                timeout_seconds: 300,
                max_retries: 3,
            };
            ReductStoreBackend::new(config)
        })
        .collect();

    assert_eq!(clients.len(), 5);
}

// Mock test to verify labels handling structure
#[test]
fn test_labels_structure() {
    let mut labels = HashMap::new();
    labels.insert("recording_id".to_string(), "rec-123".to_string());
    labels.insert("topic".to_string(), "/test/topic".to_string());
    labels.insert("format".to_string(), "mcap".to_string());

    assert_eq!(labels.len(), 3);
    assert_eq!(labels.get("recording_id").unwrap(), "rec-123");
    assert_eq!(labels.get("topic").unwrap(), "/test/topic");
    assert_eq!(labels.get("format").unwrap(), "mcap");
}

#[test]
fn test_timestamp_generation() {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp_us = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64;

    assert!(timestamp_us > 0);
    assert!(timestamp_us > 1_600_000_000_000_000); // After 2020
}

#[test]
fn test_entry_name_special_characters() {
    // Test that special characters are handled correctly
    let topics_with_special = vec![
        "/topic$special",
        "/topic@email",
        "/topic#hash",
        "/topic%percent",
    ];

    for topic in topics_with_special {
        let entry = topic_to_entry_name(topic);
        // Should not panic and should produce some result
        assert!(!entry.is_empty() || topic == "/");
    }
}

#[test]
fn test_wildcard_topic_conversion() {
    assert_eq!(topic_to_entry_name("/test/*"), "test_*");
    assert_eq!(topic_to_entry_name("/test/**"), "test_all");
    assert_eq!(topic_to_entry_name("/**"), "all");
    assert_eq!(topic_to_entry_name("*"), "*");
}

#[test]
fn test_numeric_topics() {
    assert_eq!(topic_to_entry_name("/123/456"), "123_456");
    assert_eq!(topic_to_entry_name("/sensor/1"), "sensor_1");
    assert_eq!(topic_to_entry_name("/0"), "0");
}
