// Integration tests for storage.rs with real ReductStore
// Requires ReductStore running on port 28383 (Docker)

use std::collections::HashMap;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use zenoh_recorder::storage::ReductStoreClient;

// Helper to get ReductStore URL from environment or use default
fn get_reductstore_url() -> String {
    env::var("REDUCTSTORE_TEST_URL").unwrap_or_else(|_| "http://127.0.0.1:28383".to_string())
}

// Helper to get test bucket name
fn get_test_bucket() -> String {
    env::var("REDUCTSTORE_TEST_BUCKET").unwrap_or_else(|_| "zenoh-recorder-test".to_string())
}

// Helper to check if ReductStore is available
async fn is_reductstore_available() -> bool {
    let url = get_reductstore_url();
    let client = reqwest::Client::new();
    let info_url = format!("{}/api/v1/info", url);
    
    match client.get(&info_url).send().await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

// Helper to get current timestamp in microseconds
fn current_timestamp_us() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64
}

// Helper to create unique entry names for tests
fn test_entry_name(test_name: &str) -> String {
    format!("test_{}", test_name)
}

#[tokio::test]
async fn test_storage_ensure_bucket() {
    if !is_reductstore_available().await {
        eprintln!("Skipping test: ReductStore not available at {}", get_reductstore_url());
        return;
    }

    let client = ReductStoreClient::new(get_reductstore_url(), get_test_bucket());
    
    // Should succeed creating or verifying bucket
    let result = client.ensure_bucket().await;
    assert!(result.is_ok(), "Failed to ensure bucket: {:?}", result.err());
}

#[tokio::test]
async fn test_storage_write_record() {
    if !is_reductstore_available().await {
        eprintln!("Skipping test: ReductStore not available");
        return;
    }

    let client = ReductStoreClient::new(get_reductstore_url(), get_test_bucket());
    client.ensure_bucket().await.expect("Failed to ensure bucket");

    let entry_name = test_entry_name("write_record");
    let timestamp_us = current_timestamp_us();
    let data = b"test data".to_vec();
    let mut labels = HashMap::new();
    labels.insert("test".to_string(), "value".to_string());

    let result = client
        .write_record(&entry_name, timestamp_us, data, labels)
        .await;

    assert!(result.is_ok(), "Failed to write record: {:?}", result.err());
}

#[tokio::test]
async fn test_storage_write_large_payload() {
    if !is_reductstore_available().await {
        eprintln!("Skipping test: ReductStore not available");
        return;
    }

    let client = ReductStoreClient::new(get_reductstore_url(), get_test_bucket());
    client.ensure_bucket().await.expect("Failed to ensure bucket");

    let entry_name = test_entry_name("write_large");
    let timestamp_us = current_timestamp_us();
    let data = vec![0u8; 1024 * 1024]; // 1MB
    let labels = HashMap::new();

    let result = client
        .write_record(&entry_name, timestamp_us, data, labels)
        .await;

    assert!(result.is_ok(), "Failed to write large payload: {:?}", result.err());
}

#[tokio::test]
async fn test_storage_write_with_multiple_labels() {
    if !is_reductstore_available().await {
        eprintln!("Skipping test: ReductStore not available");
        return;
    }

    let client = ReductStoreClient::new(get_reductstore_url(), get_test_bucket());
    client.ensure_bucket().await.expect("Failed to ensure bucket");

    let entry_name = test_entry_name("write_labels");
    let timestamp_us = current_timestamp_us();
    let data = b"test data with labels".to_vec();
    
    let mut labels = HashMap::new();
    labels.insert("recording_id".to_string(), "rec-001".to_string());
    labels.insert("topic".to_string(), "test/topic".to_string());
    labels.insert("compression".to_string(), "zstd".to_string());

    let result = client
        .write_record(&entry_name, timestamp_us, data, labels)
        .await;

    assert!(result.is_ok(), "Failed to write with labels: {:?}", result.err());
}

#[tokio::test]
async fn test_storage_write_multiple_records() {
    if !is_reductstore_available().await {
        eprintln!("Skipping test: ReductStore not available");
        return;
    }

    let client = ReductStoreClient::new(get_reductstore_url(), get_test_bucket());
    client.ensure_bucket().await.expect("Failed to ensure bucket");

    let entry_name = test_entry_name("write_multiple");
    let labels = HashMap::new();

    // Write 10 records with different timestamps
    for i in 0..10 {
        let timestamp_us = current_timestamp_us() + (i * 1000); // Offset by milliseconds
        let data = format!("record {}", i).into_bytes();
        
        let result = client
            .write_record(&entry_name, timestamp_us, data, labels.clone())
            .await;
        
        assert!(result.is_ok(), "Failed to write record {}: {:?}", i, result.err());
    }
}

#[tokio::test]
async fn test_storage_write_binary_data() {
    if !is_reductstore_available().await {
        eprintln!("Skipping test: ReductStore not available");
        return;
    }

    let client = ReductStoreClient::new(get_reductstore_url(), get_test_bucket());
    client.ensure_bucket().await.expect("Failed to ensure bucket");

    let entry_name = test_entry_name("write_binary");
    let timestamp_us = current_timestamp_us();
    // Binary data with all byte values
    let data: Vec<u8> = (0..=255).collect();
    let labels = HashMap::new();

    let result = client
        .write_record(&entry_name, timestamp_us, data, labels)
        .await;

    assert!(result.is_ok(), "Failed to write binary data: {:?}", result.err());
}

#[tokio::test]
async fn test_storage_write_empty_data() {
    if !is_reductstore_available().await {
        eprintln!("Skipping test: ReductStore not available");
        return;
    }

    let client = ReductStoreClient::new(get_reductstore_url(), get_test_bucket());
    client.ensure_bucket().await.expect("Failed to ensure bucket");

    let entry_name = test_entry_name("write_empty");
    let timestamp_us = current_timestamp_us();
    let data = Vec::new(); // Empty data
    let labels = HashMap::new();

    let result = client
        .write_record(&entry_name, timestamp_us, data, labels)
        .await;

    assert!(result.is_ok(), "Failed to write empty data: {:?}", result.err());
}

#[tokio::test]
async fn test_storage_write_record_with_retry_success() {
    if !is_reductstore_available().await {
        eprintln!("Skipping test: ReductStore not available");
        return;
    }

    let client = ReductStoreClient::new(get_reductstore_url(), get_test_bucket());
    client.ensure_bucket().await.expect("Failed to ensure bucket");

    let entry_name = test_entry_name("write_retry");
    let timestamp_us = current_timestamp_us();
    let data = b"test data with retry".to_vec();
    let labels = HashMap::new();

    let result = client
        .write_record_with_retry(&entry_name, timestamp_us, data, labels, 3)
        .await;

    assert!(result.is_ok(), "Failed to write with retry: {:?}", result.err());
}

#[tokio::test]
async fn test_storage_write_to_different_entries() {
    if !is_reductstore_available().await {
        eprintln!("Skipping test: ReductStore not available");
        return;
    }

    let client = ReductStoreClient::new(get_reductstore_url(), get_test_bucket());
    client.ensure_bucket().await.expect("Failed to ensure bucket");

    let entries = vec![
        "test_entry_1",
        "test_entry_2",
        "test_entry_3",
    ];

    let timestamp_us = current_timestamp_us();
    let data = b"test data".to_vec();
    let labels = HashMap::new();

    for entry in entries {
        let result = client
            .write_record(entry, timestamp_us, data.clone(), labels.clone())
            .await;
        
        assert!(result.is_ok(), "Failed to write to {}: {:?}", entry, result.err());
    }
}

#[tokio::test]
async fn test_storage_write_with_special_characters_in_labels() {
    if !is_reductstore_available().await {
        eprintln!("Skipping test: ReductStore not available");
        return;
    }

    let client = ReductStoreClient::new(get_reductstore_url(), get_test_bucket());
    client.ensure_bucket().await.expect("Failed to ensure bucket");

    let entry_name = test_entry_name("write_special_labels");
    let timestamp_us = current_timestamp_us();
    let data = b"test data".to_vec();
    
    let mut labels = HashMap::new();
    labels.insert("label-with-dash".to_string(), "value-1".to_string());
    labels.insert("label_with_underscore".to_string(), "value_2".to_string());
    labels.insert("labelWithCamelCase".to_string(), "value3".to_string());

    let result = client
        .write_record(&entry_name, timestamp_us, data, labels)
        .await;

    assert!(result.is_ok(), "Failed to write with special label names: {:?}", result.err());
}

#[tokio::test]
async fn test_storage_concurrent_writes() {
    if !is_reductstore_available().await {
        eprintln!("Skipping test: ReductStore not available");
        return;
    }

    let client = ReductStoreClient::new(get_reductstore_url(), get_test_bucket());
    client.ensure_bucket().await.expect("Failed to ensure bucket");

    let entry_name = test_entry_name("concurrent_writes");
    let labels = HashMap::new();

    // Spawn 10 concurrent writes
    let mut handles = vec![];
    for i in 0..10 {
        let client_clone = ReductStoreClient::new(get_reductstore_url(), get_test_bucket());
        let entry_name_clone = entry_name.clone();
        let labels_clone = labels.clone();
        
        let handle = tokio::spawn(async move {
            let timestamp_us = current_timestamp_us() + (i * 1000);
            let data = format!("concurrent record {}", i).into_bytes();
            
            client_clone
                .write_record(&entry_name_clone, timestamp_us, data, labels_clone)
                .await
        });
        
        handles.push(handle);
    }

    // Wait for all writes to complete
    for (i, handle) in handles.into_iter().enumerate() {
        let result = handle.await.expect("Task panicked");
        assert!(result.is_ok(), "Concurrent write {} failed: {:?}", i, result.err());
    }
}

#[tokio::test]
async fn test_storage_write_with_various_timestamps() {
    if !is_reductstore_available().await {
        eprintln!("Skipping test: ReductStore not available");
        return;
    }

    let client = ReductStoreClient::new(get_reductstore_url(), get_test_bucket());
    client.ensure_bucket().await.expect("Failed to ensure bucket");

    let data = b"test data".to_vec();
    let labels = HashMap::new();

    // Test various timestamp values - use unique entry names to avoid conflicts
    let test_cases = vec![
        (test_entry_name("timestamp_old"), current_timestamp_us() - 1_000_000_000), // 1000 seconds ago
        (test_entry_name("timestamp_now"), current_timestamp_us()),
        (test_entry_name("timestamp_future"), current_timestamp_us() + 1_000_000), // 1 second in future
    ];

    for (i, (entry_name, timestamp_us)) in test_cases.into_iter().enumerate() {
        let result = client
            .write_record(&entry_name, timestamp_us, data.clone(), labels.clone())
            .await;
        
        assert!(result.is_ok(), "Failed to write with timestamp {}: {:?}", i, result.err());
    }
}

#[tokio::test]
async fn test_storage_multiple_buckets() {
    if !is_reductstore_available().await {
        eprintln!("Skipping test: ReductStore not available");
        return;
    }

    // Create clients for different buckets
    let bucket1 = format!("{}-1", get_test_bucket());
    let bucket2 = format!("{}-2", get_test_bucket());

    let client1 = ReductStoreClient::new(get_reductstore_url(), bucket1);
    let client2 = ReductStoreClient::new(get_reductstore_url(), bucket2);

    // Ensure both buckets exist
    assert!(client1.ensure_bucket().await.is_ok());
    assert!(client2.ensure_bucket().await.is_ok());

    // Write to both buckets
    let entry_name = "test_entry";
    let timestamp_us = current_timestamp_us();
    let data = b"test data".to_vec();
    let labels = HashMap::new();

    assert!(client1.write_record(entry_name, timestamp_us, data.clone(), labels.clone()).await.is_ok());
    assert!(client2.write_record(entry_name, timestamp_us, data, labels).await.is_ok());
}

#[tokio::test]
async fn test_storage_write_with_long_entry_name() {
    if !is_reductstore_available().await {
        eprintln!("Skipping test: ReductStore not available");
        return;
    }

    let client = ReductStoreClient::new(get_reductstore_url(), get_test_bucket());
    client.ensure_bucket().await.expect("Failed to ensure bucket");

    let entry_name = format!("test_entry_with_very_long_name_{}", "x".repeat(100));
    let timestamp_us = current_timestamp_us();
    let data = b"test data".to_vec();
    let labels = HashMap::new();

    let result = client
        .write_record(&entry_name, timestamp_us, data, labels)
        .await;

    assert!(result.is_ok(), "Failed to write with long entry name: {:?}", result.err());
}

