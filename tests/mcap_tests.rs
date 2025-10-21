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
