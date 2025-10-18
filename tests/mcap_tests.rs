use zenoh_recorder::mcap_writer::McapSerializer;
use zenoh_recorder::protocol::{CompressionLevel, CompressionType};

#[test]
fn test_mcap_serializer_creation() {
    let _serializer = McapSerializer::new(CompressionType::Zstd, CompressionLevel::Default);
    // Just verify it can be created
}

#[test]
fn test_mcap_serializer_compression_none() {
    let _serializer = McapSerializer::new(CompressionType::None, CompressionLevel::Default);
}

#[test]
fn test_mcap_serializer_compression_lz4() {
    let _serializer = McapSerializer::new(CompressionType::Lz4, CompressionLevel::Fast);
}

#[test]
fn test_mcap_serializer_all_compression_levels() {
    let levels = [
        CompressionLevel::Fastest,
        CompressionLevel::Fast,
        CompressionLevel::Default,
        CompressionLevel::Slow,
        CompressionLevel::Slowest,
    ];

    for level in levels {
        let _serializer = McapSerializer::new(CompressionType::Zstd, level);
    }
}
