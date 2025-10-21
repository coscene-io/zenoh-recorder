# Zenoh Recorder Configuration Files

This directory contains configuration files for the zenoh-recorder application.

## Configuration Files

### `default.yaml`
The default configuration file with standard settings suitable for most use cases.

**Features**:
- ReductStore backend on localhost:8383
- 10 MB buffer size / 10 second flush triggers
- Zstd compression (level 2)
- 4 flush workers
- INFO log level

**Usage**:
```bash
zenoh-recorder --config config/default.yaml
```

### `examples/reductstore.yaml`
Example configuration for ReductStore backend with environment variable support.

**Features**:
- Configurable ReductStore URL via `REDUCTSTORE_URL`
- API token support via `REDUCT_API_TOKEN`
- Standard flush and compression settings

**Usage**:
```bash
export REDUCTSTORE_URL=http://production:8383
export REDUCT_API_TOKEN=your-token-here
zenoh-recorder --config config/examples/reductstore.yaml
```

### `examples/filesystem.yaml`
Example configuration for filesystem backend (Phase 3).

**Features**:
- Writes MCAP files to local filesystem
- No compression (MCAP already compressed)
- Suitable for offline/edge scenarios

**Usage**:
```bash
zenoh-recorder --config config/examples/filesystem.yaml
```

### `examples/high-performance.yaml`
Optimized configuration for high-throughput recording scenarios.

**Features**:
- 50 MB buffer size (larger batches)
- 5 second flush interval (faster)
- LZ4 compression (fastest)
- 8 flush workers (more parallelism)
- Per-topic compression optimizations
- WARN log level (less overhead)

**Usage**:
```bash
zenoh-recorder --config config/examples/high-performance.yaml
```

---

## Configuration Structure

### Zenoh Section
```yaml
zenoh:
  mode: peer  # peer, client, or router
  connect:
    endpoints:
      - tcp/localhost:7447
  listen:
    endpoints:
      - tcp/0.0.0.0:17447  # Optional
```

### Storage Section
```yaml
storage:
  backend: reductstore  # reductstore, filesystem, influxdb, s3
  reductstore:
    url: http://localhost:8383
    bucket_name: zenoh_recordings
    api_token: ${REDUCT_API_TOKEN}  # Optional, from env var
    timeout_seconds: 300
    max_retries: 3
```

### Recorder Section
```yaml
recorder:
  device_id: ${DEVICE_ID:-recorder-001}
  
  # Flush triggers
  flush_policy:
    max_buffer_size_bytes: 10485760      # 10 MB
    max_buffer_duration_seconds: 10      # 10 seconds
    min_samples_per_flush: 10
  
  # Compression settings
  compression:
    default_type: zstd  # none, lz4, zstd
    default_level: 2    # 0-4 (fastest to slowest)
    
    # Per-topic overrides (optional)
    per_topic:
      "/camera/**":
        type: lz4
        level: 1
      "/lidar/**":
        type: zstd
        level: 3
  
  # Worker pool
  workers:
    flush_workers: 4      # Concurrent flush operations
    queue_capacity: 1000  # Max pending tasks
  
  # Control interface
  control:
    key_prefix: recorder/control
    status_key: recorder/status/**
    timeout_seconds: 30

# Logging
logging:
  level: info  # trace, debug, info, warn, error
  format: text  # text, json
```

---

## Environment Variables

All configuration values support environment variable substitution:

### Syntax
```yaml
# Simple substitution
device_id: ${DEVICE_ID}

# With default value
device_id: ${DEVICE_ID:-robot-001}
```

### Common Environment Variables
- `DEVICE_ID` - Device identifier
- `REDUCTSTORE_URL` - ReductStore server URL
- `REDUCT_API_TOKEN` - ReductStore API authentication token
- `BUCKET_NAME` - Storage bucket name

---

## Creating Custom Configurations

### 1. Copy and Modify
```bash
# Copy default config
cp config/default.yaml my-config.yaml

# Edit with your settings
vim my-config.yaml

# Run with custom config
zenoh-recorder --config my-config.yaml
```

### 2. Override Specific Values
```bash
# Use default config but override device ID
zenoh-recorder --config config/default.yaml --device-id robot-042
```

### 3. Use Environment Variables
```bash
# Set environment variables
export DEVICE_ID=robot-123
export REDUCTSTORE_URL=http://production:8383

# Run with environment variable substitution
zenoh-recorder --config config/default.yaml
```

---

## Validation

Configurations are validated on load. Common validation rules:

- **Buffer size** must be > 0
- **Flush duration** must be > 0
- **Compression level** must be 0-4
- **Worker count** must be > 0
- **Queue capacity** must be > 0
- **Device ID** cannot be empty
- **Backend** must be supported (currently: reductstore, filesystem)

---

## Tips

### Performance Tuning

**High-throughput scenarios**:
- Increase `max_buffer_size_bytes` (e.g., 50 MB)
- Decrease `max_buffer_duration_seconds` (e.g., 5 seconds)
- Use LZ4 compression (faster than zstd)
- Increase `flush_workers` (e.g., 8)
- Set log level to `warn` or `error`

**Low-latency scenarios**:
- Decrease `max_buffer_duration_seconds` (e.g., 1 second)
- Decrease `max_buffer_size_bytes` (e.g., 1 MB)
- Use `none` or `lz4` compression
- Increase `flush_workers`

**Resource-constrained devices**:
- Decrease `max_buffer_size_bytes` (e.g., 5 MB)
- Use LZ4 compression or none
- Decrease `flush_workers` (e.g., 2)
- Set log level to `warn`

### Per-Topic Compression

Optimize compression based on data characteristics:

```yaml
per_topic:
  "/camera/**":
    type: lz4
    level: 1  # Fast for high-frequency camera data
  
  "/lidar/**":
    type: zstd
    level: 3  # Better compression for lidar
  
  "/imu/**":
    type: none
    level: 0  # No compression for small IMU data
  
  "/logs/**":
    type: zstd
    level: 4  # Maximum compression for text logs
```

---

## Troubleshooting

### Config file not found
```bash
# Check file path
ls -la config/default.yaml

# Use absolute path
zenoh-recorder --config /absolute/path/to/config.yaml
```

### Environment variable not substituted
```bash
# Verify environment variable is set
echo $DEVICE_ID

# Check syntax in config file
# Correct: ${DEVICE_ID}
# Correct: ${DEVICE_ID:-default}
# Incorrect: $DEVICE_ID
```

### Validation errors
```bash
# Read error message carefully
zenoh-recorder --config my-config.yaml
# Error: max_buffer_size_bytes must be > 0

# Fix the invalid value in config file
```

---

## Support

For more information:
- [Design Documentation](../docs/CONFIG_AND_STORAGE_DESIGN.md)
- [Implementation Guide](../docs/CONFIG_IMPLEMENTATION.md)
- [Main README](../README.md)

---

**Quick Start**: Use `config/default.yaml` for getting started, then customize as needed!


