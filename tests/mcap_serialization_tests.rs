use zenoh::key_expr::KeyExpr;
use zenoh::sample::Sample;
use zenoh_recorder::mcap_writer::McapSerializer;
use zenoh_recorder::protocol::{CompressionLevel, CompressionType};

// Helper function to create samples
fn create_sample(topic: &'static str, data: Vec<u8>) -> Sample {
    let key: KeyExpr<'static> = topic.try_into().unwrap();
    Sample::new(key, data)
}

#[test]
fn test_serialize_empty_batch() {
    let serializer = McapSerializer::new(CompressionType::None, CompressionLevel::Default);
    let result = serializer
        .serialize_batch("/test/topic", vec![], "rec-123")
        .unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_serialize_single_sample() {
    let serializer = McapSerializer::new(CompressionType::None, CompressionLevel::Default);
    let sample = create_sample("test/topic", b"test payload".to_vec());

    let result = serializer
        .serialize_batch("/test/topic", vec![sample], "rec-123")
        .unwrap();

    assert!(!result.is_empty());
    // Should contain header
    let result_str = String::from_utf8_lossy(&result[..100.min(result.len())]);
    assert!(result_str.contains("ZENOH_MCAP"));
}

#[test]
fn test_serialize_multiple_samples() {
    let serializer = McapSerializer::new(CompressionType::None, CompressionLevel::Default);

    let samples: Vec<Sample> = (0..10)
        .map(|i| create_sample("test/topic", format!("payload_{}", i).into_bytes()))
        .collect();

    let result = serializer
        .serialize_batch("/test/topic", samples, "rec-123")
        .unwrap();

    assert!(!result.is_empty());
    // Check header contains correct count
    let header_end = result.iter().position(|&b| b == b'\n').unwrap();
    let header = String::from_utf8_lossy(&result[..header_end]);
    assert!(header.contains("count=10"));
}

#[test]
fn test_serialize_with_lz4_compression() {
    let serializer = McapSerializer::new(CompressionType::Lz4, CompressionLevel::Fast);

    let samples: Vec<Sample> = (0..50)
        .map(|i| create_sample("test/topic", format!("payload_with_data_{}", i).into_bytes()))
        .collect();

    let result = serializer
        .serialize_batch("/test/topic", samples, "rec-123")
        .unwrap();

    assert!(!result.is_empty());
}

#[test]
fn test_serialize_with_zstd_compression() {
    let serializer = McapSerializer::new(CompressionType::Zstd, CompressionLevel::Default);

    let samples: Vec<Sample> = (0..50)
        .map(|i| create_sample("test/topic", format!("test_payload_data_{}", i).into_bytes()))
        .collect();

    let result = serializer
        .serialize_batch("/test/topic", samples, "rec-123")
        .unwrap();

    assert!(!result.is_empty());
}

#[test]
fn test_compression_levels_zstd() {
    let levels = [
        CompressionLevel::Fastest,
        CompressionLevel::Fast,
        CompressionLevel::Default,
        CompressionLevel::Slow,
        CompressionLevel::Slowest,
    ];

    // Create repeated data for better compression
    let repeated_data = "test data ".repeat(100);
    let samples: Vec<Sample> = (0..10)
        .map(|_| create_sample("test/topic", repeated_data.as_bytes().to_vec()))
        .collect();

    let mut sizes = Vec::new();
    for level in levels {
        let serializer = McapSerializer::new(CompressionType::Zstd, level);
        let result = serializer
            .serialize_batch("/test/topic", samples.clone(), "rec-123")
            .unwrap();
        sizes.push(result.len());
    }

    // Verify all succeeded
    assert_eq!(sizes.len(), 5);
    // All should produce output
    assert!(sizes.iter().all(|&s| s > 0));
}

#[test]
fn test_compression_levels_lz4() {
    let levels = [
        CompressionLevel::Fastest,
        CompressionLevel::Default,
        CompressionLevel::Slowest,
    ];

    let samples: Vec<Sample> = (0..20)
        .map(|i| create_sample("test/topic", format!("sample_data_{}", i).into_bytes()))
        .collect();

    for level in levels {
        let serializer = McapSerializer::new(CompressionType::Lz4, level);
        let result = serializer
            .serialize_batch("/test/topic", samples.clone(), "rec-123")
            .unwrap();
        assert!(!result.is_empty());
    }
}

#[test]
fn test_large_payload() {
    let serializer = McapSerializer::new(CompressionType::Zstd, CompressionLevel::Fast);

    // Create a large payload
    let large_data = vec![0u8; 1024 * 1024]; // 1 MB
    let data_size = large_data.len();
    let sample = create_sample("test/topic", large_data);

    let result = serializer
        .serialize_batch("/test/topic", vec![sample], "rec-123")
        .unwrap();

    assert!(!result.is_empty());
    // Compression should reduce size significantly for zeros
    assert!(result.len() < data_size);
}

#[test]
fn test_different_topics() {
    let serializer = McapSerializer::new(CompressionType::None, CompressionLevel::Default);

    // Test each topic separately with its own static key
    let sample1 = create_sample("camera/front", b"data".to_vec());
    let result1 = serializer
        .serialize_batch("/camera/front", vec![sample1], "rec-123")
        .unwrap();
    assert!(!result1.is_empty());
    assert!(String::from_utf8_lossy(&result1).contains("camera/front"));

    let sample2 = create_sample("lidar/points", b"data".to_vec());
    let result2 = serializer
        .serialize_batch("/lidar/points", vec![sample2], "rec-123")
        .unwrap();
    assert!(!result2.is_empty());

    let sample3 = create_sample("imu/data", b"data".to_vec());
    let result3 = serializer
        .serialize_batch("/imu/data", vec![sample3], "rec-123")
        .unwrap();
    assert!(!result3.is_empty());
}

#[test]
fn test_compression_ratio() {
    let serializer_none = McapSerializer::new(CompressionType::None, CompressionLevel::Default);
    let serializer_zstd = McapSerializer::new(CompressionType::Zstd, CompressionLevel::Default);

    // Create highly compressible data
    let repeated = "a".repeat(10000);
    let samples: Vec<Sample> = (0..10)
        .map(|_| create_sample("test/topic", repeated.as_bytes().to_vec()))
        .collect();

    let uncompressed = serializer_none
        .serialize_batch("/test/topic", samples.clone(), "rec-123")
        .unwrap();
    
    let compressed = serializer_zstd
        .serialize_batch("/test/topic", samples, "rec-123")
        .unwrap();

    // Compressed should be significantly smaller
    assert!(compressed.len() < uncompressed.len());
    let ratio = uncompressed.len() as f64 / compressed.len() as f64;
    assert!(ratio > 2.0, "Compression ratio should be > 2x for repeated data");
}

#[test]
fn test_binary_payload() {
    let serializer = McapSerializer::new(CompressionType::None, CompressionLevel::Default);

    // Create binary data
    let binary_data: Vec<u8> = (0..256).map(|i| i as u8).collect();
    let sample = create_sample("test/topic", binary_data);

    let result = serializer
        .serialize_batch("/test/topic", vec![sample], "rec-123")
        .unwrap();

    assert!(!result.is_empty());
}

#[test]
fn test_recording_id_in_output() {
    let serializer = McapSerializer::new(CompressionType::None, CompressionLevel::Default);
    let sample = create_sample("test/topic", b"data".to_vec());

    let result = serializer
        .serialize_batch("/test/topic", vec![sample], "unique-rec-id-456")
        .unwrap();

    let result_str = String::from_utf8_lossy(&result);
    assert!(result_str.contains("unique-rec-id-456"));
}


