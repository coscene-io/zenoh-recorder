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

/// Production-ready MCAP serializer for Zenoh samples with protobuf encoding
///
/// This module provides high-performance serialization of Zenoh samples to a custom
/// MCAP-compatible format with protobuf message encoding and optional compression.
///
/// # Format Structure
///
/// Each serialized batch contains:
/// - Header with metadata (topic, recording_id, sample count)
/// - Length-prefixed protobuf messages
/// - Optional compression (LZ4 or Zstd)
///
/// # Performance
///
/// - Zero-copy where possible using Zenoh's buffer API
/// - Efficient protobuf encoding via prost
/// - SIMD-accelerated compression (via native libraries)
///
use anyhow::{Context, Result};
use prost::Message;
use std::io::Write;
use tracing::debug;
use zenoh::prelude::SplitBuffer;
use zenoh::sample::Sample;

use crate::config::SchemaConfig;
use crate::protocol::{CompressionLevel, CompressionType};

/// MCAP writer that serializes Zenoh samples into compressed protobuf format
///
/// # Thread Safety
///
/// This type is Send + Sync and can be used across multiple threads.
///
/// # Examples
///
/// ```ignore
/// use zenoh_recorder::mcap_writer::McapSerializer;
/// use zenoh_recorder::protocol::{CompressionType, CompressionLevel};
///
/// let serializer = McapSerializer::new(
///     CompressionType::Zstd,
///     CompressionLevel::Default,
/// );
/// ```
pub struct McapSerializer {
    compression_type: CompressionType,
    compression_level: CompressionLevel,
    schema_config: SchemaConfig,
}

impl McapSerializer {
    /// Create a new MCAP serializer with specified compression settings
    ///
    /// # Arguments
    ///
    /// * `compression_type` - Type of compression to apply (None, LZ4, Zstd)
    /// * `compression_level` - Compression level (Fastest to Slowest)
    ///
    /// # Performance Notes
    ///
    /// - LZ4: Fast compression, moderate ratio (~2-3x)
    /// - Zstd: Slower but better compression (~4-6x)
    /// - None: No compression overhead, largest size
    pub fn new(compression_type: CompressionType, compression_level: CompressionLevel) -> Self {
        Self {
            compression_type,
            compression_level,
            schema_config: SchemaConfig::default(),
        }
    }
    
    /// Create a new MCAP serializer with schema configuration
    pub fn with_schema_config(
        compression_type: CompressionType,
        compression_level: CompressionLevel,
        schema_config: SchemaConfig,
    ) -> Self {
        Self {
            compression_type,
            compression_level,
            schema_config,
        }
    }
    
    /// Get schema info for a topic
    fn get_schema_info(&self, topic: &str) -> Option<crate::proto::SchemaInfo> {
        if !self.schema_config.include_metadata {
            return None;
        }
        
        // Check per-topic schema config
        if let Some(topic_schema) = self.schema_config.per_topic.get(topic) {
            return Some(crate::proto::SchemaInfo {
                format: topic_schema.format.clone(),
                schema_name: topic_schema.schema_name.clone().unwrap_or_default(),
                schema_hash: topic_schema.schema_hash.clone().unwrap_or_default(),
                schema_data: vec![],
            });
        }
        
        // Use default format if metadata is enabled
        Some(crate::proto::SchemaInfo {
            format: self.schema_config.default_format.clone(),
            schema_name: String::new(),
            schema_hash: String::new(),
            schema_data: vec![],
        })
    }

    /// Serialize a batch of samples to protobuf-encoded format
    ///
    /// This method:
    /// 1. Wraps each sample in a protobuf SensorData message
    /// 2. Encodes all messages with length prefixes
    /// 3. Adds a header with metadata
    /// 4. Applies compression if configured
    ///
    /// # Arguments
    ///
    /// * `topic` - Zenoh topic name
    /// * `samples` - Vector of samples to serialize
    /// * `recording_id` - Unique recording identifier for metadata
    ///
    /// # Returns
    ///
    /// Compressed binary data ready for storage
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Protobuf encoding fails
    /// - Compression fails
    /// - I/O error during buffering
    ///
    /// # Performance
    ///
    /// Time complexity: O(n * m) where n = sample count, m = avg sample size
    /// Space complexity: O(total_size + compression_overhead)
    pub fn serialize_batch(
        &self,
        topic: &str,
        samples: Vec<Sample>,
        recording_id: &str,
    ) -> Result<Vec<u8>> {
        if samples.is_empty() {
            debug!("Empty sample batch for topic '{}'", topic);
            return Ok(Vec::new());
        }

        let mut all_messages = Vec::with_capacity(samples.len());
        let mut total_payload_size = 0usize;

        // Encode all samples to protobuf
        for sample in &samples {
            let timestamp = sample
                .timestamp
                .as_ref()
                .map(|ts| ts.get_time().as_u64())
                .unwrap_or_else(|| {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as u64
                });

            // Create generic protobuf message from sample (schema-agnostic)
            let schema_info = self.get_schema_info(topic);
            let recorded_msg = crate::proto::RecordedMessage {
                topic: topic.to_string(),
                timestamp_ns: timestamp as i64,
                payload: sample.payload.contiguous().to_vec(),
                schema: schema_info,
            };

            let mut msg_data = Vec::new();
            recorded_msg
                .encode(&mut msg_data)
                .context("Failed to encode protobuf message")?;

            total_payload_size += msg_data.len();
            all_messages.push(msg_data);
        }

        // Pre-allocate buffer based on estimated size
        let estimated_size = total_payload_size + (all_messages.len() * 4) + 256; // +4 bytes per length prefix, +256 for header
        let mut buffer = Vec::with_capacity(estimated_size);

        // Write header with metadata
        self.write_header(&mut buffer, topic, recording_id, samples.len())?;

        // Write all messages with length prefixes
        for msg in &all_messages {
            // Write length prefix (4 bytes, little-endian)
            buffer.extend_from_slice(&(msg.len() as u32).to_le_bytes());
            // Write message data
            buffer.extend_from_slice(msg);
        }

        let uncompressed_size = buffer.len();

        debug!(
            "Serialized {} samples to protobuf format ({} bytes uncompressed)",
            samples.len(),
            uncompressed_size
        );

        // Apply compression
        let compressed = self.compress(buffer)?;

        debug!(
            "Compressed data from {} to {} bytes using {:?} (ratio: {:.2}x)",
            uncompressed_size,
            compressed.len(),
            self.compression_type,
            uncompressed_size as f64 / compressed.len().max(1) as f64
        );

        Ok(compressed)
    }

    /// Write format header with metadata
    ///
    /// Header format (ASCII text for debugging):
    /// ```text
    /// ZENOH_MCAP|topic={topic}|recording_id={id}|count={n}\n
    /// ```
    fn write_header(
        &self,
        buffer: &mut Vec<u8>,
        topic: &str,
        recording_id: &str,
        count: usize,
    ) -> Result<()> {
        writeln!(
            buffer,
            "ZENOH_MCAP|topic={}|recording_id={}|count={}",
            topic, recording_id, count
        )
        .context("Failed to write header")
    }

    /// Compress data based on configured compression type
    ///
    /// # Performance
    ///
    /// - LZ4: ~500 MB/s compression, ~2 GB/s decompression
    /// - Zstd: ~100-200 MB/s compression, ~500 MB/s decompression
    fn compress(&self, data: Vec<u8>) -> Result<Vec<u8>> {
        match self.compression_type {
            CompressionType::None => Ok(data),
            CompressionType::Lz4 => self.compress_lz4(data),
            CompressionType::Zstd => self.compress_zstd(data),
        }
    }

    /// Compress using LZ4 algorithm
    ///
    /// LZ4 provides very fast compression/decompression with moderate compression ratio.
    /// Ideal for real-time recording where CPU is a bottleneck.
    fn compress_lz4(&self, data: Vec<u8>) -> Result<Vec<u8>> {
        let level = self.compression_level.to_lz4_level();
        let mut encoder = lz4::EncoderBuilder::new()
            .level(level)
            .build(Vec::new())
            .context("Failed to create LZ4 encoder")?;

        encoder
            .write_all(&data)
            .context("Failed to write data to LZ4 encoder")?;

        let (compressed, result) = encoder.finish();
        result.context("LZ4 compression failed")?;

        Ok(compressed)
    }

    /// Compress using Zstd algorithm
    ///
    /// Zstd provides excellent compression ratio with good speed.
    /// Ideal for archival or when network bandwidth is limited.
    ///
    /// # Implementation Notes
    ///
    /// Uses zstd-rs which wraps the native C library with SIMD optimizations.
    fn compress_zstd(&self, data: Vec<u8>) -> Result<Vec<u8>> {
        let level = self.compression_level.to_zstd_level();
        zstd::encode_all(&data[..], level).context("Zstd compression failed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serializer_creation() {
        let _ = McapSerializer::new(CompressionType::Zstd, CompressionLevel::Default);
        let _ = McapSerializer::new(CompressionType::Lz4, CompressionLevel::Fast);
        let _ = McapSerializer::new(CompressionType::None, CompressionLevel::Fastest);
    }

    #[test]
    fn test_header_format() {
        let serializer = McapSerializer::new(CompressionType::None, CompressionLevel::Default);
        let mut buffer = Vec::new();
        serializer
            .write_header(&mut buffer, "/test/topic", "rec-123", 42)
            .unwrap();

        let header = String::from_utf8(buffer).unwrap();
        assert!(header.contains("ZENOH_MCAP"));
        assert!(header.contains("topic=/test/topic"));
        assert!(header.contains("recording_id=rec-123"));
        assert!(header.contains("count=42"));
    }

    #[test]
    fn test_empty_batch() {
        let serializer = McapSerializer::new(CompressionType::None, CompressionLevel::Default);
        let result = serializer
            .serialize_batch("/test", vec![], "rec-123")
            .unwrap();
        assert!(result.is_empty());
    }
}
