# Zenoh Recorder with ReductStore Backend

A high-performance data recorder for Zenoh middleware that:
- Records multi-topic data streams
- Flushes data based on size or time thresholds
- Serializes data to MCAP format with protobuf messages
- Stores data in ReductStore with configurable compression
- Supports distributed recording control via request-response protocol

## Features

- **Multi-topic Recording**: Subscribe to multiple Zenoh topics simultaneously
- **MCAP Format**: Industry-standard container format for time-series data
- **Protobuf Serialization**: Efficient binary serialization
- **Compression**: LZ4 and Zstd compression support
- **Double Buffering**: Non-blocking writes while flushing
- **Size/Time Based Flushing**: Configurable flush policies
- **ReductStore Backend**: Cloud-native time-series database
- **Request-Response Protocol**: Control recordings via Zenoh queries
- **Retry Logic**: Automatic retry with exponential backoff

## Architecture

```
Zenoh Network → Subscribers → Lock-free Queues → Topic Buffers
                                                       ↓
                                              Double Buffers
                                                       ↓
                                              Flush Workers
                                                       ↓
                                              MCAP Serializer
                                                       ↓
                                              ReductStore Client
                                                       ↓
                                              ReductStore
```

## Prerequisites

1. **Zenoh**: Ensure Zenoh router is running
2. **ReductStore**: Run ReductStore locally or remotely

### Start ReductStore (Docker)

```bash
docker run -d -p 8383:8383 reduct/store:latest
```

## Building

```bash
cd zenoh-recorder-example
cargo build --release
```

## Running

```bash
# Set environment variables (optional)
export DEVICE_ID="robot_01"
export REDUCTSTORE_URL="http://localhost:8383"
export BUCKET_NAME="ros_data"

# Run the recorder
cargo run --release
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

### Flush Policies

Configurable in code (to be moved to config file):

```rust
let buffer = TopicBuffer::new(
    topic.clone(),
    recording_id.clone(),
    10 * 1024 * 1024,        // 10 MB max buffer size
    Duration::from_secs(10), // 10 second max duration
    flush_queue.clone(),
);
```

### Compression Options

- **Type**: `none`, `lz4`, `zstd`
- **Level**: 0 (fastest) to 4 (slowest/best compression)

### Worker Threads

Default: 4 flush worker threads (configurable in `RecorderManager::start_flush_workers`)

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

1. **Buffer Sizes**: Increase for high-throughput topics
2. **Flush Duration**: Decrease for lower latency, increase for better batching
3. **Worker Threads**: Increase for more parallel uploads
4. **Compression**: Use LZ4 for speed, Zstd for size
5. **Network**: Use HTTP/2 connection pooling (enabled by default)

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

### No data being recorded
- Check if topics are being published
- Verify Zenoh session is connected
- Check logs for subscription errors

### Upload failures
- Verify ReductStore is running: `curl http://localhost:8383/api/v1/info`
- Check network connectivity
- Review retry logs

### High memory usage
- Reduce buffer sizes
- Decrease flush duration
- Increase number of flush workers

## Future Enhancements

- [ ] Configuration file support (YAML/JSON)
- [ ] Prometheus metrics exporter
- [ ] Local disk spooling for offline operation
- [ ] Data replay functionality
- [ ] Query by time range
- [ ] Multi-format support (Parquet, Arrow)
- [ ] Data filtering and downsampling

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

## See Also

- [Zenoh Documentation](https://zenoh.io/docs/)
- [ReductStore Documentation](https://www.reduct.store/docs)
- [MCAP Format](https://mcap.dev/)
- [Design Document](../RECORDER_DESIGN.md)

