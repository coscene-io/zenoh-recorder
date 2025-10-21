# Zenoh Recorder

A high-performance, write-only data recorder for Zenoh middleware with **multi-backend storage support**.

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

## Table of Contents

- [Overview](#overview)
- [Quick Start](#-quick-start)
- [Features](#features)
- [Architecture](#architecture)
- [Prerequisites](#prerequisites)
- [Building](#building)
- [Running](#running)
- [Usage Examples](#usage-examples)
- [Configuration](#configuration)
- [Custom Proto Definitions](#custom-proto-definitions)
- [Supported Backends](#supported-backends)
- [Performance Tuning](#performance-tuning)
- [Troubleshooting](#troubleshooting)
- [Documentation](#documentation)
- [License](#license)

## Overview

The Zenoh Recorder is a lightweight agent that:
- 📊 Records multi-topic data streams from Zenoh
- ⚙️ **Configurable flush triggers** (size & time based)
- 📦 Serializes to MCAP format (schema-agnostic)
- 🎨 **Custom proto support** - use ANY serialization format (protobuf, JSON, msgpack, etc.)
- 🔌 **Supports multiple storage backends** (ReductStore, Filesystem, InfluxDB, S3)
- 🎯 **TOML configuration** with environment variable support
- 🚀 High-performance with **configurable worker pools**
- 🎛️ Distributed recording control via request-response protocol
- 🔄 Automatic retry logic with exponential backoff

## 🆕 What's New in v0.1

This release introduces a complete configuration and multi-backend storage system:

- ✅ **TOML Configuration**: All settings externalized to config files
- ✅ **Multi-Backend Support**: Trait-based storage abstraction (ReductStore, Filesystem)
- ✅ **Custom Proto Support**: Schema-agnostic recording - use ANY serialization format
- ✅ **Schema Metadata**: Optional per-topic schema information
- ✅ **Flexible Flush Policies**: Configure size and time triggers
- ✅ **Per-Topic Compression**: Optimize compression per data type
- ✅ **Worker Pools**: Configurable parallelism
- ✅ **Environment Variables**: `${VAR:-default}` substitution support

**Migration Note**: Existing code continues to work via backward-compatible API.

## 🚀 Quick Start

```bash
# 1. Clone the repository
git clone https://github.com/coscene-io/zenoh-recorder.git
cd zenoh-recorder

# 2. Install protoc (required for building)
# Debian/Ubuntu
sudo apt-get update && sudo apt-get install -y protobuf-compiler
# macOS: brew install protobuf

# 3. Build
cargo build --release

# 4. Start infrastructure (Docker)
docker run -d -p 7447:7447 eclipse/zenoh:latest
docker run -d -p 8383:8383 reduct/store:latest

# 5. Run recorder with default config
./target/release/zenoh-recorder --config config/default.toml

# 6. Start a recording (in another terminal)
echo '{
  "command": "start",
  "device_id": "robot-001",
  "topics": ["/test/data"],
  "compression_type": "zstd",
  "compression_level": 2
}' | z_put 'recorder/control/recorder-001'

# 7. Query data in ReductStore Web UI
open http://localhost:8383
```

For a complete deployment example, see `examples/docker-compose.yml`.

## Features

### Core Capabilities
- **Multi-topic Recording**: Subscribe to multiple Zenoh topics simultaneously
- **MCAP Format**: Industry-standard container format for time-series data
- **Protobuf Serialization**: Efficient binary serialization
- **Compression**: LZ4 and Zstd compression support (per-topic configurable)
- **Double Buffering**: Non-blocking writes while flushing
- **Size/Time Based Flushing**: Fully configurable flush policies
- **Request-Response Protocol**: Control recordings via Zenoh queries
- **Retry Logic**: Automatic retry with exponential backoff

### 🆕 Configuration System
- **TOML Configuration**: Externalized configuration files
- **Environment Variables**: `${VAR}` and `${VAR:-default}` substitution
- **CLI Arguments**: Override config values via command line
- **Validation**: Automatic config validation on startup
- **Per-Topic Settings**: Customize compression per topic pattern

### 🔌 Multi-Backend Storage
- **Pluggable Backends**: Trait-based storage abstraction
- **ReductStore**: Time-series database (production ready)
- **Filesystem**: MCAP files to disk (production ready)
- **InfluxDB**: Metrics and analytics (coming soon)
- **S3**: Cloud archival (coming soon)
- **Easy to Extend**: Implement `StorageBackend` trait for new backends

### 🚀 Performance
- **Configurable Workers**: Tune parallelism for your workload
- **Lock-free Queues**: Minimize contention
- **Connection Pooling**: HTTP/2 connection reuse
- **SIMD Compression**: Hardware-accelerated compression

## Architecture

### Write-Only Agent Design

```
┌──────────────────────────────────────────────────────┐
│  Zenoh Recorder (Write-Only Agent)                   │
│                                                       │
│  Zenoh Subscribers → Topic Buffers → Flush Workers   │
│                           ↓               ↓           │
│                    Double Buffers   MCAP Serializer   │
│                           ↓               ↓           │
│                   Size/Time Triggers  Compression     │
│                                           ↓           │
│                                  StorageBackend Trait │
└───────────────────────────────────────────┬───────────┘
                                            │
                        ┌───────────────────┴──────────────────┐
                        │   Backend (User Selects via Config)  │
                        │                                       │
                        │  • ReductStore (time-series)          │
                        │  • Filesystem (MCAP files)            │
                        │  • InfluxDB (metrics)                 │
                        │  • S3 (cloud archive)                 │
                        └───────────────────────────────────────┘
                                            ↓
                        ┌───────────────────────────────────────┐
                        │   Query Tools (Backend-Specific)      │
                        │                                       │
                        │  • ReductStore Web UI / API           │
                        │  • Grafana Dashboards                 │
                        │  • MCAP Tools / Foxglove Studio       │
                        │  • S3 Select / Athena                 │
                        └───────────────────────────────────────┘
```

**Key Principle**: Recorder writes, users query backend directly using specialized tools.

## Prerequisites

### Required
1. **Rust**: 1.75 or later
2. **Protocol Buffers Compiler (protoc)**: Required for building
   - **Debian/Ubuntu**: `sudo apt-get install protobuf-compiler`
   - **macOS**: `brew install protobuf`
   - **Arch Linux**: `sudo pacman -S protobuf`
   - **Windows**: Download from [protobuf releases](https://github.com/protocolbuffers/protobuf/releases)
   - Alternatively, set `PROTOC` env variable to the path of `protoc` binary
3. **Zenoh**: Zenoh router or peer network

### Storage Backend (Choose One or More)
- **ReductStore** (recommended for time-series data)
  ```bash
  docker run -d -p 8383:8383 reduct/store:latest
  ```
- **Filesystem** (production ready - no external service needed)
- **InfluxDB** (coming soon)
- **S3** (coming soon)

### Quick Start with Docker Compose

We provide a complete example with Zenoh + ReductStore + Recorder:

```bash
cd zenoh-recorder/examples
docker-compose up -d
```

This starts:
- Zenoh router on port 7447
- ReductStore on port 8383
- Zenoh Recorder agent

## Building

### Prerequisites Check

Before building, ensure you have `protoc` installed:

```bash
# Check if protoc is installed
protoc --version

# If not installed:
# Debian/Ubuntu
sudo apt-get update && sudo apt-get install -y protobuf-compiler

# macOS
brew install protobuf

# Arch Linux
sudo pacman -S protobuf
```

### Build Commands

```bash
cd zenoh-recorder
cargo build --release
```

## Running

### Option 1: With Configuration File (Recommended)

```bash
# Use default configuration
./target/release/zenoh-recorder --config config/default.toml

# Or specify custom config
./target/release/zenoh-recorder --config my-config.toml

# Override device ID
./target/release/zenoh-recorder --config config/default.toml --device-id robot-042
```

### Option 2: With Environment Variables

```bash
export DEVICE_ID="robot_01"
export REDUCTSTORE_URL="http://localhost:8383"
export REDUCT_API_TOKEN="optional-token"

./target/release/zenoh-recorder --config config/default.toml
```

## Usage Examples

### 1. Start a Recording

Using `curl` or any HTTP client with Zenoh:

```bash
# Using z_put (Zenoh CLI tool)
echo '{
  "command": "start",
  "scene": "highway_driving",
  "skills": ["lane_keeping", "obstacle_avoidance"],
  "organization": "acme_robotics",
  "task_id": "task-001",
  "device_id": "robot_01",
  "data_collector_id": "collector-01",
  "topics": ["/camera/front", "/lidar/points", "/imu/data"],
  "compression_level": 2,
  "compression_type": "zstd"
}' | z_put 'recorder/control/robot_01'
```

Response:
```json
{
  "success": true,
  "message": "Operation completed successfully",
  "recording_id": "550e8400-e29b-41d4-a716-446655440000",
  "bucket_name": "ros_data"
}
```

### 2. Query Recording Status

```bash
z_get 'recorder/status/550e8400-e29b-41d4-a716-446655440000'
```

Response:
```json
{
  "success": true,
  "message": "Status retrieved successfully",
  "status": "recording",
  "scene": "highway_driving",
  "skills": ["lane_keeping", "obstacle_avoidance"],
  "organization": "acme_robotics",
  "task_id": "task-001",
  "device_id": "robot_01",
  "data_collector_id": "collector-01",
  "active_topics": ["/camera/front", "/lidar/points", "/imu/data"],
  "buffer_size_bytes": 5242880,
  "total_recorded_bytes": 104857600
}
```

### 3. Pause/Resume Recording

```bash
# Pause
echo '{
  "command": "pause",
  "recording_id": "550e8400-e29b-41d4-a716-446655440000",
  "device_id": "robot_01"
}' | z_put 'recorder/control/robot_01'

# Resume
echo '{
  "command": "resume",
  "recording_id": "550e8400-e29b-41d4-a716-446655440000",
  "device_id": "robot_01"
}' | z_put 'recorder/control/robot_01'
```

### 4. Finish Recording

```bash
echo '{
  "command": "finish",
  "recording_id": "550e8400-e29b-41d4-a716-446655440000",
  "device_id": "robot_01"
}' | z_put 'recorder/control/robot_01'
```

## Configuration

### TOML Configuration File

Create a `config.toml` file:

```toml
# Zenoh connection
[zenoh]
mode = "peer"  # peer, client, or router

[zenoh.connect]
endpoints = [
    "tcp/localhost:7447"
]

# Storage backend selection
[storage]
backend = "reductstore"  # reductstore, filesystem, influxdb, s3

[storage.reductstore]
url = "http://localhost:8383"
bucket_name = "zenoh_recordings"
api_token = "${REDUCT_API_TOKEN}"  # Optional
timeout_seconds = 300
max_retries = 3

# Recorder settings
[recorder]
device_id = "${DEVICE_ID:-robot-001}"

# Flush triggers (NEW!)
[recorder.flush_policy]
max_buffer_size_bytes = 10485760      # 10 MB
max_buffer_duration_seconds = 10      # 10 seconds
min_samples_per_flush = 10

# Compression settings (NEW!)
[recorder.compression]
default_type = "zstd"  # none, lz4, zstd
default_level = 2      # 0-4

# Per-topic overrides (optional)
[recorder.compression.per_topic."/camera/**"]
type = "lz4"
level = 1  # Fast compression for high-frequency camera

[recorder.compression.per_topic."/lidar/**"]
type = "zstd"
level = 3  # Better compression for lidar

# Worker configuration (NEW!)
[recorder.workers]
flush_workers = 4       # Parallel flush operations
queue_capacity = 1000   # Task queue size

# Control interface
[recorder.control]
key_prefix = "recorder/control"
status_key = "recorder/status/**"

# Logging
[logging]
level = "info"  # trace, debug, info, warn, error
format = "text"
```

### Configuration Examples

See `config/examples/` for more examples:
- `reductstore.toml` - ReductStore backend
- `filesystem.toml` - Filesystem backend
- `high-performance.toml` - Optimized for throughput

For detailed configuration options, see [config/README.md](config/README.md).

## Custom Proto Definitions

The recorder is **schema-agnostic** - it stores raw Zenoh payloads without making assumptions about the serialization format. This means you can use **your own protobuf definitions** (or any serialization format) without recompiling the recorder.

### How It Works

```
┌─────────────────────────────────────────────────────────────┐
│  Your Application (Publisher)                                │
│                                                               │
│  1. Define your own proto:                                   │
│     message MyCustomMessage {                                │
│       string sensor_id = 1;                                  │
│       double temperature = 2;                                │
│     }                                                         │
│                                                               │
│  2. Serialize it yourself:                                   │
│     let data = MyCustomMessage { ... };                      │
│     let bytes = data.encode_to_vec();                        │
│                                                               │
│  3. Publish to Zenoh:                                        │
│     session.put("/sensors/temp", bytes).await;               │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│  Zenoh Recorder (Storage)                                    │
│                                                               │
│  - Stores raw bytes (no deserialization)                     │
│  - Optionally adds schema metadata                           │
│  - Works with ANY serialization format                       │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│  Your Application (Consumer)                                 │
│                                                               │
│  1. Query data from storage backend                          │
│  2. Deserialize with your proto:                             │
│     let data = storage.get(...);                             │
│     let msg = MyCustomMessage::decode(data.payload);         │
└─────────────────────────────────────────────────────────────┘
```

### Example: Using Custom Proto

**Step 1: Define your proto** (in your application)

```rust
// In your own crate - NOT in the recorder
#[derive(Clone, prost::Message)]
pub struct MyCustomMessage {
    #[prost(string, tag = "1")]
    pub sensor_id: String,
    
    #[prost(double, tag = "2")]
    pub temperature: f64,
    
    #[prost(int64, tag = "3")]
    pub timestamp_ms: i64,
}
```

**Step 2: Publish your data**

```rust
use zenoh::prelude::*;
use prost::Message;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create your custom message
    let my_data = MyCustomMessage {
        sensor_id: "DHT22-001".to_string(),
        temperature: 23.5,
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
    };
    
    // Serialize it yourself
    let bytes = my_data.encode_to_vec();
    
    // Publish to Zenoh
    let session = zenoh::open(config::default()).res().await?;
    session.put("/sensors/temperature", bytes).res().await?;
    
    Ok(())
}
```

**Step 3: Configure recorder with schema metadata** (optional)

```toml
# config.toml
[recorder.schema]
# Enable schema metadata in recordings
include_metadata = true

# Specify schema info per topic
[recorder.schema.per_topic."/sensors/temperature"]
format = "protobuf"
schema_name = "my_package.MyCustomMessage"
schema_hash = "v1.0.0"  # Optional version
```

**Step 4: Query and deserialize**

```rust
// Later, when reading the data
use prost::Message;

// Get data from storage (e.g., ReductStore, filesystem)
let recorded_data = storage.get("/sensors/temperature").await?;

// Deserialize with YOUR proto definition
let my_msg = MyCustomMessage::decode(recorded_data.payload.as_slice())?;

println!("Sensor: {}, Temp: {}", my_msg.sensor_id, my_msg.temperature);
```

### Supported Serialization Formats

The recorder is format-agnostic and supports:

| Format | Description | Use Case |
|--------|-------------|----------|
| **Protobuf** | Binary, schema-based | Recommended for structured data |
| **JSON** | Text, human-readable | Easy debugging, web APIs |
| **MessagePack** | Binary, schemaless | Compact, dynamic data |
| **FlatBuffers** | Zero-copy binary | Ultra-low latency |
| **Raw Binary** | Custom formats | Full control |
| **CBOR** | Binary JSON alternative | IoT devices |

**Example: Using JSON**

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct SensorData {
    sensor_id: String,
    temperature: f64,
}

// Publish
let data = SensorData { sensor_id: "S001".into(), temperature: 25.3 };
let json = serde_json::to_vec(&data)?;
session.put("/sensors/temp", json).await?;

// Configure schema metadata
// per_topic:
//   "/sensors/temp":
//     format: json
//     schema_name: SensorData
```

### Schema Metadata Benefits

When you enable schema metadata, the recorder stores additional information:

```toml
[recorder.schema]
include_metadata = true

[recorder.schema.per_topic."/camera/image"]
format = "protobuf"
schema_name = "sensor_msgs.Image"
schema_hash = "a1b2c3d4e5f6"  # SHA hash of .proto file
```

**Benefits:**
- ✅ **Documentation** - Know what format each topic uses
- ✅ **Versioning** - Track schema changes via hash
- ✅ **Validation** - Verify data compatibility
- ✅ **Tooling** - Auto-generate deserializers

**Stored metadata:**
```json
{
  "topic": "/camera/image",
  "timestamp_ns": 1234567890,
  "payload": "<raw bytes>",
  "schema": {
    "format": "protobuf",
    "schema_name": "sensor_msgs.Image",
    "schema_hash": "a1b2c3d4e5f6"
  }
}
```

### Example Configurations

**Minimal (no schema metadata):**
```toml
[recorder.schema]
default_format = "raw"
include_metadata = false  # Default
```

**With schema metadata:**
```toml
[recorder.schema]
default_format = "protobuf"
include_metadata = true

[recorder.schema.per_topic."/camera/**"]
format = "protobuf"
schema_name = "sensor_msgs.Image"

[recorder.schema.per_topic."/telemetry/**"]
format = "json"
```

See [config/examples/schema-enabled.toml](config/examples/schema-enabled.toml) for a complete example.

### Key Advantages

✅ **No recompilation** - Recorder doesn't need to know your proto definitions  
✅ **Any format** - Protobuf, JSON, msgpack, custom binary, etc.  
✅ **Flexibility** - Change schemas without updating recorder  
✅ **User control** - You manage serialization in your application  
✅ **Backward compatible** - Existing workflows continue to work  
✅ **Performance** - Zero overhead from schema inspection  

### Example Code

See [examples/custom_proto_usage.rs](examples/custom_proto_usage.rs) for a complete working example.

## ReductStore Data Structure

```
Bucket: "ros_data"
│
├─── Entry: "recordings_metadata"
│     ├── Record @ timestamp_1
│     │   Data: {recording_id, topics, scene, ...}
│     │   Labels: {recording_id, device_id, scene}
│     │
│     └── Record @ timestamp_2
│         Data: {...}
│         Labels: {...}
│
├─── Entry: "camera_front"
│     ├── Record @ timestamp_1
│     │   Data: MCAP file (100 messages)
│     │   Labels: {recording_id, topic, format: "mcap"}
│     │
│     └── Record @ timestamp_2
│         Data: MCAP file (100 messages)
│         Labels: {recording_id, topic, format: "mcap"}
│
├─── Entry: "lidar_points"
│     └── ...
│
└─── Entry: "imu_data"
      └── ...
```

## Performance Tuning

All performance settings are now configurable via TOML:

### High-Throughput Scenario

```toml
[recorder.flush_policy]
max_buffer_size_bytes = 52428800  # 50 MB (larger batches)
max_buffer_duration_seconds = 5   # Faster flush

[recorder.compression]
default_type = "lz4"  # Faster compression
default_level = 1

[recorder.workers]
flush_workers = 8      # More parallelism
queue_capacity = 2000

[logging]
level = "warn"  # Less overhead
```

### Low-Latency Scenario

```toml
[recorder.flush_policy]
max_buffer_size_bytes = 1048576   # 1 MB (smaller batches)
max_buffer_duration_seconds = 1   # Immediate flush

[recorder.compression]
default_type = "none"  # No compression overhead

[recorder.workers]
flush_workers = 2
```

### Resource-Constrained Devices

```toml
[recorder.flush_policy]
max_buffer_size_bytes = 5242880   # 5 MB
max_buffer_duration_seconds = 10

[recorder.compression]
default_type = "lz4"  # Fast compression
default_level = 1

[recorder.workers]
flush_workers = 2      # Fewer workers
queue_capacity = 500

[logging]
level = "warn"
```

### Per-Topic Optimization

```toml
[recorder.compression]
default_type = "zstd"
default_level = 2

[recorder.compression.per_topic."/camera/**"]
type = "lz4"
level = 1  # Fast for high-frequency camera

[recorder.compression.per_topic."/lidar/**"]
type = "zstd"
level = 3  # Better compression for lidar

[recorder.compression.per_topic."/imu/**"]
type = "none"  # No compression for small IMU data
level = 0
```

See `config/examples/high-performance.toml` for a complete optimized configuration.

## Testing

### Start Test Publishers

```bash
# Terminal 1: Publish to /camera/front
z_pub '/camera/front' --payload "camera_data_frame_001"

# Terminal 2: Publish to /lidar/points
z_pub '/lidar/points' --payload "lidar_pointcloud_001"

# Terminal 3: Publish to /imu/data
z_pub '/imu/data' --payload "imu_acceleration_001"
```

### Query ReductStore

```bash
# List entries
curl http://localhost:8383/api/v1/b/ros_data

# Query metadata
curl http://localhost:8383/api/v1/b/ros_data/recordings_metadata

# Query camera data
curl http://localhost:8383/api/v1/b/ros_data/camera_front
```

## Troubleshooting

### Build Issues

**`protoc` not found error**
```
Error: Custom { kind: NotFound, error: "Could not find `protoc`..." }
```

**Solution:**
```bash
# Install protoc
# Debian/Ubuntu
sudo apt-get update && sudo apt-get install -y protobuf-compiler

# macOS
brew install protobuf

# Verify installation
protoc --version

# Alternative: Set PROTOC environment variable
export PROTOC=/path/to/protoc
cargo build --release
```

### Configuration Issues

**Config file not found**
```bash
# Verify file path
ls -la config/default.toml

# Use absolute path
zenoh-recorder --config /absolute/path/to/config.toml
```

**Environment variable not substituted**
```bash
# Verify variable is set
echo $DEVICE_ID

# Correct syntax in config file:
# ✅ ${DEVICE_ID}
# ✅ ${DEVICE_ID:-default-value}
# ❌ $DEVICE_ID (wrong)
```

**Validation errors**
```bash
# Read error message carefully
zenoh-recorder --config my-config.toml
# Error: max_buffer_size_bytes must be > 0

# Fix the invalid value in config file
```

### No Data Being Recorded
- Check if topics are being published: `z_pub /test/topic "test data"`
- Verify Zenoh session is connected (check logs)
- Check logs for subscription errors
- Verify recording is started (check status)

### Upload Failures
- Verify backend is running:
  - ReductStore: `curl http://localhost:8383/api/v1/info`
  - Filesystem: Check disk space and permissions
- Check network connectivity
- Review retry logs (increase log level to `debug`)
- Check backend authentication (API tokens)

### High Memory Usage
- Reduce `max_buffer_size_bytes` in config
- Decrease `max_buffer_duration_seconds`
- Increase `flush_workers` for faster processing
- Use lighter compression (LZ4 or none)
- Check for slow backend writes (bottleneck)

### Performance Issues
- **Slow writes**: Increase `flush_workers`, use LZ4 compression
- **High CPU**: Reduce compression level, use LZ4 instead of Zstd
- **Network saturation**: Enable compression, increase buffer size
- **Disk I/O**: Use SSD, increase worker count

### Debug Mode

Enable detailed logging:
```toml
[logging]
level = "debug"  # or trace
format = "text"
```

Or via environment:
```bash
RUST_LOG=zenoh_recorder=debug ./target/release/zenoh-recorder --config config/default.toml
```

## Supported Backends

### ✅ ReductStore (Production Ready)
**Best for**: Time-series data, robotics, IoT

- Time-series optimized storage
- Built-in retention policies
- Web UI for data exploration
- HTTP API for queries
- Label-based metadata

**Query with**: ReductStore Web UI at `http://localhost:8383` or HTTP API

### ✅ Filesystem (Production Ready)
**Best for**: Offline recording, edge devices

- Writes MCAP files to local disk
- No external dependencies
- Automatic directory organization by entry name
- JSON metadata files for labels
- Query with: MCAP tools or Foxglove Studio

### 🔜 InfluxDB (Coming Soon)
**Best for**: Metrics, analytics, dashboards

- Time-series database for metrics
- Grafana integration
- Powerful query language (InfluxQL)

### 🔜 S3 (Coming Soon)
**Best for**: Cloud archival, long-term storage

- Serverless cloud storage
- Query with Athena or S3 Select
- Cost-effective archival

### 🛠️ Custom Backends

Easy to add! Just implement the `StorageBackend` trait:

```rust
#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn initialize(&self) -> Result<()>;
    async fn write_record(...) -> Result<()>;
    async fn write_with_retry(...) -> Result<()>;
    async fn health_check(&self) -> Result<bool>;
    fn backend_type(&self) -> &str;
}
```

See [docs/CONFIG_AND_STORAGE_DESIGN.md](docs/CONFIG_AND_STORAGE_DESIGN.md) for details.

### Backend Comparison

| Feature | ReductStore | Filesystem | InfluxDB | S3 |
|---------|-------------|------------|----------|-----|
| **Status** | ✅ Ready | ✅ Ready | 🔜 Soon | 🔜 Soon |
| **Best For** | Time-series | Edge/Offline | Metrics | Archive |
| **Query UI** | Web UI | Foxglove | Grafana | Athena |
| **Setup** | Docker | None | Docker | Cloud |
| **Retention** | Built-in | Manual | Built-in | Lifecycle |
| **Cost** | Low | None | Medium | Pay-per-GB |
| **Latency** | Low | Lowest | Low | High |
| **Scalability** | High | Limited | High | Unlimited |

## Recent Enhancements

- [x] **TOML configuration system** with environment variables
- [x] **Multi-backend storage** via trait abstraction
- [x] **Configurable flush triggers** (size & time)
- [x] **Per-topic compression** settings
- [x] **Configurable worker pools**
- [x] **CLI with config file support**
- [x] **Comprehensive documentation**
- [x] **Filesystem backend implementation**

## Future Enhancements

- [ ] InfluxDB backend implementation
- [ ] S3 backend implementation
- [ ] Multi-backend writes (primary + fallback)
- [ ] Prometheus metrics exporter
- [ ] Local disk spooling for offline operation
- [ ] Data replay functionality
- [ ] Multi-format support (Parquet, Arrow)
- [ ] Data filtering and downsampling
- [ ] Hot config reload

## License

Copyright 2025 coScene

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

## Documentation

- 📖 [Configuration Guide](config/README.md) - Configuration file reference
- 🏗️ [Design Document](docs/CONFIG_AND_STORAGE_DESIGN.md) - Architecture and design decisions
- 🔧 [Contributing Guide](docs/CONTRIBUTING.md) - Testing guide
- 🚀 [CI/CD Documentation](docs/CI_CD.md) - Continuous integration setup
- 📊 [Recorder Design](docs/RECORDER_DESIGN.md) - Detailed technical design

## See Also

### Project Resources
- [Configuration Examples](config/examples/) - Example TOML configs
- [Docker Compose Example](examples/docker-compose.yml) - Complete deployment example

### External Documentation
- [Zenoh Documentation](https://zenoh.io/docs/)
- [ReductStore Documentation](https://www.reduct.store/docs)
- [MCAP Format](https://mcap.dev/)
- [Foxglove Studio](https://foxglove.dev/) - MCAP visualization

## Quick Links

| Topic | Link |
|-------|------|
| **Getting Started** | See [Prerequisites](#prerequisites) and [Building](#building) |
| **Configuration** | See [config/README.md](config/README.md) |
| **Backend Selection** | See [Supported Backends](#supported-backends) |
| **Performance Tuning** | See [Performance Tuning](#performance-tuning) |
| **API Reference** | Run `cargo doc --open` |
| **Design Docs** | See [docs/](docs/) directory |

