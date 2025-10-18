/// Comprehensive storage tests
///
/// Tests for ReductStore client and HTTP operations
///
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use zenoh_recorder::storage::{topic_to_entry_name, ReductStoreClient};

#[test]
fn test_client_creation_various_urls() {
    let urls = vec![
        "http://localhost:8383",
        "http://192.168.1.100:8383",
        "http://reductstore.example.com",
        "https://secure.reductstore.io",
        "http://10.0.0.1:9090",
    ];

    for url in urls {
        let client = ReductStoreClient::new(url.to_string(), "bucket".to_string());
        drop(client);
    }
}

#[test]
fn test_client_creation_various_buckets() {
    let buckets = vec![
        "simple_bucket",
        "bucket-with-dash",
        "bucket_with_underscore",
        "bucket123",
        "my-data-2024",
    ];

    for bucket in buckets {
        let client = ReductStoreClient::new(
            "http://localhost:8383".to_string(),
            bucket.to_string(),
        );
        drop(client);
    }
}

#[test]
fn test_topic_conversion_comprehensive() {
    // Standard cases
    assert_eq!(topic_to_entry_name("/camera/front"), "camera_front");
    assert_eq!(topic_to_entry_name("/camera/back"), "camera_back");
    assert_eq!(topic_to_entry_name("/lidar/top"), "lidar_top");
    
    // Deep nesting
    assert_eq!(topic_to_entry_name("/a/b/c/d/e/f"), "a_b_c_d_e_f");
    
    // Special characters
    assert_eq!(topic_to_entry_name("/topic-dash"), "topic-dash");
    assert_eq!(topic_to_entry_name("/topic_underscore"), "topic_underscore");
    assert_eq!(topic_to_entry_name("/topic.dot"), "topic.dot");
    
    // Numbers
    assert_eq!(topic_to_entry_name("/sensor/1"), "sensor_1");
    assert_eq!(topic_to_entry_name("/123/456"), "123_456");
    
    // Wildcards
    assert_eq!(topic_to_entry_name("/test/*"), "test_*");
    assert_eq!(topic_to_entry_name("/test/**"), "test_all");
    assert_eq!(topic_to_entry_name("/**"), "all");
    
    // Edge cases
    assert_eq!(topic_to_entry_name(""), "");
    assert_eq!(topic_to_entry_name("/"), "");
    assert_eq!(topic_to_entry_name("topic"), "topic");
}

#[test]
fn test_topic_conversion_idempotent() {
    let topics = vec![
        "/camera/front",
        "/lidar/points",
        "/imu/data",
        "/test/**",
    ];

    for topic in topics {
        let result1 = topic_to_entry_name(topic);
        let result2 = topic_to_entry_name(topic);
        assert_eq!(result1, result2, "Conversion should be idempotent for {}", topic);
    }
}

#[test]
fn test_labels_creation() {
    let mut labels = HashMap::new();
    labels.insert("recording_id".to_string(), "rec-001".to_string());
    labels.insert("topic".to_string(), "/camera/front".to_string());
    labels.insert("format".to_string(), "mcap".to_string());
    labels.insert("device_id".to_string(), "robot-01".to_string());

    assert_eq!(labels.len(), 4);
    assert!(labels.contains_key("recording_id"));
    assert!(labels.contains_key("topic"));
    assert!(labels.contains_key("format"));
    assert!(labels.contains_key("device_id"));
}

#[test]
fn test_timestamp_microseconds() {
    let timestamp_us = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64;

    // Should be a reasonable timestamp
    assert!(timestamp_us > 1_600_000_000_000_000); // After 2020
    assert!(timestamp_us < 2_000_000_000_000_000); // Before 2033
}

#[test]
fn test_timestamp_precision() {
    let ts1 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64;

    std::thread::sleep(std::time::Duration::from_micros(100));

    let ts2 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64;

    // Second timestamp should be later
    assert!(ts2 > ts1);
    // Difference should be at least 100 microseconds
    assert!(ts2 - ts1 >= 100);
}

#[test]
fn test_entry_names_no_collision() {
    // These should all produce different entry names
    let topics = vec![
        "/camera/front",
        "/camera/back",
        "/lidar/points",
        "/imu/data",
        "/gps/location",
    ];

    let mut entry_names = std::collections::HashSet::new();
    for topic in topics {
        let entry = topic_to_entry_name(topic);
        assert!(
            entry_names.insert(entry.clone()),
            "Collision detected for topic: {}",
            topic
        );
    }

    assert_eq!(entry_names.len(), 5);
}

#[test]
fn test_topic_conversion_preserves_info() {
    // Entry name should contain enough info to reconstruct topic
    let topic = "/camera/front/left";
    let entry = topic_to_entry_name(topic);
    
    assert!(entry.contains("camera"));
    assert!(entry.contains("front"));
    assert!(entry.contains("left"));
}

#[test]
fn test_reductstore_url_handling() {
    let urls = vec![
        ("http://localhost:8383", "bucket"),
        ("https://example.com:443", "data"),
        ("http://192.168.1.1:9090", "sensors"),
    ];

    for (url, bucket) in urls {
        let _client = ReductStoreClient::new(url.to_string(), bucket.to_string());
        // Just verify creation doesn't panic
    }
}

#[test]
fn test_labels_serialization() {
    let mut labels = HashMap::new();
    labels.insert("key1".to_string(), "value1".to_string());
    labels.insert("key2".to_string(), "value2".to_string());

    let json = serde_json::to_string(&labels).unwrap();
    let deserialized: HashMap<String, String> = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.len(), 2);
    assert_eq!(deserialized.get("key1").unwrap(), "value1");
    assert_eq!(deserialized.get("key2").unwrap(), "value2");
}

#[test]
fn test_complex_topic_names() {
    let complex_topics = vec![
        ("/robot/arm/joint_1/position", "robot_arm_joint_1_position"),
        ("/robot/arm/joint_2/velocity", "robot_arm_joint_2_velocity"),
        ("/sensor/lidar/scan/filtered", "sensor_lidar_scan_filtered"),
        ("/nav/goal/waypoint/current", "nav_goal_waypoint_current"),
    ];

    for (topic, expected) in complex_topics {
        assert_eq!(topic_to_entry_name(topic), expected);
    }
}

#[test]
fn test_topic_with_multiple_slashes() {
    assert_eq!(topic_to_entry_name("///test"), "test");
    assert_eq!(topic_to_entry_name("/test///topic"), "test___topic");
    assert_eq!(topic_to_entry_name("test////data"), "test____data");
}

#[test]
fn test_very_long_topic_names() {
    let long_topic = format!("/{}", "segment/".repeat(20));
    let entry = topic_to_entry_name(&long_topic);
    
    assert!(entry.len() > 0);
    assert!(entry.contains("segment"));
}

#[test]
fn test_unicode_in_topics() {
    // ReductStore may or may not support Unicode, but conversion shouldn't panic
    let topics = vec![
        "/test/日本語",
        "/тест/topic",
        "/test/🚀",
    ];

    for topic in topics {
        let _entry = topic_to_entry_name(topic);
        // Should not panic
    }
}

