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

// Edge case tests to improve coverage for mcap_writer.rs and other modules

use zenoh::prelude::r#async::*;
use zenoh_recorder::mcap_writer::McapSerializer;
use zenoh_recorder::protocol::{CompressionLevel, CompressionType};

fn create_sample(data: Vec<u8>) -> Sample {
    Sample::new(KeyExpr::try_from("test/topic").unwrap(), data)
}

#[test]
fn test_mcap_compression_with_maximum_level() {
    let serializer = McapSerializer::new(CompressionType::Zstd, CompressionLevel::Slowest);

    let samples = vec![create_sample(vec![0u8; 1000])];
    let result = serializer.serialize_batch("test_topic", samples, "test_recording");
    assert!(result.is_ok(), "Should handle maximum compression level");
}

#[test]
fn test_mcap_with_empty_topic_name() {
    let serializer = McapSerializer::new(CompressionType::None, CompressionLevel::Default);

    let samples = vec![create_sample(vec![1, 2, 3])];
    let result = serializer.serialize_batch("", samples, "test_recording");
    assert!(result.is_ok(), "Should handle empty topic name");
}

#[test]
fn test_mcap_with_very_small_samples() {
    let serializer = McapSerializer::new(CompressionType::Lz4, CompressionLevel::Fastest);

    let mut samples = Vec::new();
    for i in 0..1000 {
        samples.push(create_sample(vec![i as u8]));
    }

    let result = serializer.serialize_batch("test_topic", samples, "test_recording");
    assert!(result.is_ok(), "Should handle many tiny samples");
}

#[test]
fn test_mcap_extreme_compression_ratio() {
    let serializer = McapSerializer::new(CompressionType::Zstd, CompressionLevel::Slowest);

    let samples = vec![create_sample(vec![0u8; 10000])];
    let result = serializer.serialize_batch("test_topic", samples, "test_recording");
    assert!(result.is_ok());

    let compressed = result.unwrap();
    assert!(
        compressed.len() < 1000,
        "Highly compressible data should compress well"
    );
}

#[test]
fn test_compression_level_zstd_mappings() {
    let levels = vec![
        (CompressionLevel::Fastest, 1),
        (CompressionLevel::Fast, 3),
        (CompressionLevel::Default, 5),
        (CompressionLevel::Slow, 10),
        (CompressionLevel::Slowest, 19),
    ];

    for (level, expected) in levels {
        assert_eq!(
            level.to_zstd_level(),
            expected,
            "Zstd level mismatch for {:?}",
            level
        );
    }
}

#[test]
fn test_compression_level_lz4_mappings() {
    let levels = vec![
        (CompressionLevel::Fastest, 1),
        (CompressionLevel::Fast, 3),
        (CompressionLevel::Default, 5),
        (CompressionLevel::Slow, 9),
        (CompressionLevel::Slowest, 12),
    ];

    for (level, expected) in levels {
        assert_eq!(
            level.to_lz4_level(),
            expected,
            "LZ4 level mismatch for {:?}",
            level
        );
    }
}

#[test]
fn test_all_compression_type_combinations() {
    let types = vec![
        CompressionType::None,
        CompressionType::Lz4,
        CompressionType::Zstd,
    ];

    let levels = vec![
        CompressionLevel::Fastest,
        CompressionLevel::Fast,
        CompressionLevel::Default,
        CompressionLevel::Slow,
        CompressionLevel::Slowest,
    ];

    for compression_type in &types {
        for compression_level in &levels {
            let serializer = McapSerializer::new(*compression_type, *compression_level);
            let samples = vec![create_sample(vec![1, 2, 3, 4, 5])];
            let result = serializer.serialize_batch("test_topic", samples, "test_recording");
            assert!(
                result.is_ok(),
                "Failed with {:?}/{:?}",
                compression_type,
                compression_level
            );
        }
    }
}

#[test]
fn test_mcap_with_special_characters_in_topic() {
    let serializer = McapSerializer::new(CompressionType::None, CompressionLevel::Default);

    let samples = vec![create_sample(vec![1, 2, 3])];
    let result = serializer.serialize_batch(
        "topic/with/special_chars-123",
        samples,
        "test-recording-001",
    );
    assert!(result.is_ok(), "Should handle special characters");
}

#[test]
fn test_mcap_with_unicode_in_recording_id() {
    let serializer = McapSerializer::new(CompressionType::None, CompressionLevel::Default);

    let samples = vec![create_sample(vec![1, 2, 3])];
    let result = serializer.serialize_batch("test_topic", samples, "测试记录");
    assert!(result.is_ok(), "Should handle Unicode");
}

#[test]
fn test_mcap_with_mixed_sample_sizes() {
    let serializer = McapSerializer::new(CompressionType::Zstd, CompressionLevel::Default);

    let samples = vec![
        create_sample(vec![1u8; 10]),
        create_sample(vec![2u8; 1000]),
        create_sample(vec![3u8; 100]),
        create_sample(vec![4u8; 10000]),
    ];

    let result = serializer.serialize_batch("test_topic", samples, "test_recording");
    assert!(result.is_ok(), "Should handle mixed sizes");
}

#[test]
fn test_mcap_with_large_single_sample() {
    let serializer = McapSerializer::new(CompressionType::Lz4, CompressionLevel::Fastest);

    let samples = vec![create_sample(vec![0u8; 1_000_000])]; // 1MB
    let result = serializer.serialize_batch("test_topic", samples, "test_recording");
    assert!(result.is_ok(), "Should handle large samples");
}

#[test]
fn test_mcap_with_binary_patterns() {
    let serializer = McapSerializer::new(CompressionType::Zstd, CompressionLevel::Default);

    let patterns = vec![
        vec![0u8; 1000],
        vec![255u8; 1000],
        (0u8..=255).cycle().take(1000).collect::<Vec<u8>>(),
        vec![0xAA; 1000],
    ];

    for pattern in patterns {
        let samples = vec![create_sample(pattern)];
        let result = serializer.serialize_batch("test_topic", samples, "test_recording");
        assert!(result.is_ok(), "Should handle various binary patterns");
    }
}

#[test]
fn test_mcap_no_compression_size() {
    let serializer = McapSerializer::new(CompressionType::None, CompressionLevel::Default);

    let samples = vec![create_sample(vec![1u8; 10000])];
    let result = serializer.serialize_batch("test_topic", samples, "test_recording");
    assert!(result.is_ok());

    let output = result.unwrap();
    assert!(
        output.len() > 10000,
        "No compression should preserve most size"
    );
}

#[test]
fn test_mcap_with_topic_edge_cases() {
    let test_cases = vec![
        ("", "empty"),
        ("a", "single char"),
        ("a/b/c/d/e/f/g/h/i/j", "deep nesting"),
        ("topic_underscore", "underscore"),
        ("topic-dash", "dash"),
        ("topic.dot", "dot"),
        ("UPPERCASE", "uppercase"),
    ];

    for (topic_name, _description) in test_cases {
        let serializer = McapSerializer::new(CompressionType::None, CompressionLevel::Default);

        let samples = vec![create_sample(vec![1, 2, 3])];
        let result = serializer.serialize_batch(topic_name, samples, "test_recording");
        assert!(result.is_ok(), "Should handle topic: {}", topic_name);
    }
}

#[test]
fn test_compression_level_ordering() {
    // Verify levels are ordered correctly
    assert!(CompressionLevel::Fastest.to_zstd_level() < CompressionLevel::Fast.to_zstd_level());
    assert!(CompressionLevel::Fast.to_zstd_level() < CompressionLevel::Default.to_zstd_level());
    assert!(CompressionLevel::Default.to_zstd_level() < CompressionLevel::Slow.to_zstd_level());
    assert!(CompressionLevel::Slow.to_zstd_level() < CompressionLevel::Slowest.to_zstd_level());
}

#[test]
fn test_mcap_with_many_samples() {
    let serializer = McapSerializer::new(CompressionType::Zstd, CompressionLevel::Fast);

    let mut samples = Vec::new();
    for i in 0..10000 {
        samples.push(create_sample(vec![i as u8; 10]));
    }

    let result = serializer.serialize_batch("test_topic", samples, "test_recording");
    assert!(result.is_ok(), "Should handle 10000 samples");
}

#[test]
fn test_mcap_compression_type_none_vs_lz4() {
    let data = vec![0u8; 10000];

    let no_compression = McapSerializer::new(CompressionType::None, CompressionLevel::Default);
    let lz4_compression = McapSerializer::new(CompressionType::Lz4, CompressionLevel::Default);

    let samples1 = vec![create_sample(data.clone())];
    let samples2 = vec![create_sample(data.clone())];

    let result1 = no_compression
        .serialize_batch("test", samples1, "rec")
        .unwrap();
    let result2 = lz4_compression
        .serialize_batch("test", samples2, "rec")
        .unwrap();

    assert!(
        result2.len() < result1.len(),
        "LZ4 should compress better than none"
    );
}

#[test]
fn test_mcap_compression_type_lz4_vs_zstd() {
    let data = vec![0u8; 10000]; // Highly compressible

    let lz4 = McapSerializer::new(CompressionType::Lz4, CompressionLevel::Default);
    let zstd = McapSerializer::new(CompressionType::Zstd, CompressionLevel::Default);

    let samples1 = vec![create_sample(data.clone())];
    let samples2 = vec![create_sample(data.clone())];

    let result1 = lz4.serialize_batch("test", samples1, "rec").unwrap();
    let result2 = zstd.serialize_batch("test", samples2, "rec").unwrap();

    // Zstd should compress better than LZ4 for highly compressible data
    assert!(
        result2.len() < result1.len(),
        "Zstd should compress better than LZ4 for repetitive data"
    );
}

#[test]
fn test_mcap_with_very_long_recording_id() {
    let serializer = McapSerializer::new(CompressionType::None, CompressionLevel::Default);

    let long_id = "x".repeat(1000);
    let samples = vec![create_sample(vec![1, 2, 3])];
    let result = serializer.serialize_batch("test", samples, &long_id);
    assert!(result.is_ok(), "Should handle very long recording ID");
}

#[test]
fn test_mcap_with_very_long_topic() {
    let serializer = McapSerializer::new(CompressionType::None, CompressionLevel::Default);

    let long_topic = "topic/".repeat(100);
    let samples = vec![create_sample(vec![1, 2, 3])];
    let result = serializer.serialize_batch(&long_topic, samples, "rec");
    assert!(result.is_ok(), "Should handle very long topic");
}
