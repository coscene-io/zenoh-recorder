// Configuration system integration tests

use zenoh_recorder::config::{load_config, RecorderConfig};
use std::fs;
use std::path::PathBuf;

#[test]
fn test_load_default_config() {
    let config_path = PathBuf::from("config/default.yaml");
    
    if config_path.exists() {
        let result = load_config(&config_path);
        assert!(result.is_ok(), "Failed to load default config: {:?}", result.err());
        
        let config = result.unwrap();
        
        // Verify defaults
        assert_eq!(config.zenoh.mode, "peer");
        assert_eq!(config.storage.backend, "reductstore");
        assert_eq!(config.recorder.flush_policy.max_buffer_size_bytes, 10485760);
        assert_eq!(config.recorder.flush_policy.max_buffer_duration_seconds, 10);
        assert_eq!(config.recorder.workers.flush_workers, 4);
        assert_eq!(config.logging.level, "info");
    }
}

#[test]
fn test_config_with_env_vars() {
    // Create temporary config file
    let temp_config = r#"
zenoh:
  mode: peer

storage:
  backend: reductstore
  reductstore:
    url: ${TEST_URL:-http://default:8383}
    bucket_name: ${TEST_BUCKET:-default_bucket}
    api_token: ${TEST_TOKEN}
    timeout_seconds: 300
    max_retries: 3

recorder:
  device_id: ${DEVICE_ID:-test-device}
  flush_policy:
    max_buffer_size_bytes: 1048576
    max_buffer_duration_seconds: 5
    min_samples_per_flush: 10
  compression:
    default_type: zstd
    default_level: 2
  workers:
    flush_workers: 2
    queue_capacity: 500

logging:
  level: debug
  format: text
"#;
    
    let temp_path = PathBuf::from("/tmp/test_config.yaml");
    fs::write(&temp_path, temp_config).expect("Failed to write temp config");
    
    // Set environment variable
    std::env::set_var("TEST_URL", "http://testhost:9000");
    std::env::set_var("DEVICE_ID", "robot-123");
    
    let result = load_config(&temp_path);
    assert!(result.is_ok(), "Failed to load config with env vars: {:?}", result.err());
    
    let config = result.unwrap();
    
    // Verify env var substitution
    if let Some(reduct_config) = config.storage.backend_config.as_reductstore() {
        assert_eq!(reduct_config.url, "http://testhost:9000");
        assert_eq!(reduct_config.bucket_name, "default_bucket"); // Uses default
    } else {
        panic!("Expected ReductStore config");
    }
    
    assert_eq!(config.recorder.device_id, "robot-123");
    assert_eq!(config.recorder.flush_policy.max_buffer_size_bytes, 1048576);
    assert_eq!(config.recorder.workers.flush_workers, 2);
    
    // Cleanup
    fs::remove_file(temp_path).ok();
    std::env::remove_var("TEST_URL");
    std::env::remove_var("DEVICE_ID");
}

#[test]
fn test_config_validation() {
    let invalid_config = r#"
zenoh:
  mode: peer

storage:
  backend: reductstore
  reductstore:
    url: http://localhost:8383
    bucket_name: test
    timeout_seconds: 300
    max_retries: 3

recorder:
  device_id: test
  flush_policy:
    max_buffer_size_bytes: 0  # INVALID: must be > 0
    max_buffer_duration_seconds: 10
  compression:
    default_type: zstd
    default_level: 2
  workers:
    flush_workers: 4
    queue_capacity: 1000

logging:
  level: info
  format: text
"#;
    
    let temp_path = PathBuf::from("/tmp/invalid_config.yaml");
    fs::write(&temp_path, invalid_config).expect("Failed to write temp config");
    
    let result = load_config(&temp_path);
    assert!(result.is_err(), "Expected validation error for invalid config");
    assert!(result.unwrap_err().to_string().contains("max_buffer_size_bytes"));
    
    // Cleanup
    fs::remove_file(temp_path).ok();
}

#[test]
fn test_backend_factory() {
    use zenoh_recorder::config::{StorageConfig, BackendConfig, ReductStoreConfig};
    use zenoh_recorder::storage::BackendFactory;
    
    let storage_config = StorageConfig {
        backend: "reductstore".to_string(),
        backend_config: BackendConfig::ReductStore {
            reductstore: ReductStoreConfig {
                url: "http://localhost:8383".to_string(),
                bucket_name: "test_bucket".to_string(),
                api_token: None,
                timeout_seconds: 300,
                max_retries: 3,
            },
        },
    };
    
    let result = BackendFactory::create(&storage_config);
    assert!(result.is_ok(), "Failed to create backend: {:?}", result.err());
    
    let backend = result.unwrap();
    assert_eq!(backend.backend_type(), "reductstore");
}

#[test]
fn test_config_defaults() {
    let config = RecorderConfig::default();
    
    assert_eq!(config.zenoh.mode, "peer");
    assert_eq!(config.storage.backend, "reductstore");
    assert_eq!(config.recorder.device_id, "recorder-001");
    assert_eq!(config.recorder.flush_policy.max_buffer_size_bytes, 10485760);
    assert_eq!(config.recorder.flush_policy.max_buffer_duration_seconds, 10);
    assert_eq!(config.recorder.workers.flush_workers, 4);
    assert_eq!(config.recorder.workers.queue_capacity, 1000);
    assert_eq!(config.logging.level, "info");
}

