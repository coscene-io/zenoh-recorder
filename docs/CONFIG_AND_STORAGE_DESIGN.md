# Configuration & Storage Backend Design

## 📋 Design Proposal: Configuration System + Multi-Backend Storage

**Date**: October 18, 2025  
**Status**: 🔄 **PROPOSAL - AWAITING APPROVAL**

---

## 🎯 Design Philosophy

**Recorder Role**: **Write-Only Data Pipeline Agent**

The recorder is a lightweight agent that runs on devices to:
- ✅ Subscribe to Zenoh topics
- ✅ Buffer and flush data efficiently
- ✅ Compress and serialize to MCAP format
- ✅ **Write data to storage backend**

**Query Happens at Backend**: Users query data directly through backend tools:
- ReductStore → Web UI, HTTP API
- InfluxDB → Grafana dashboards
- Filesystem → MCAP tools, Foxglove Studio
- S3 → Athena, download tools

**Key Principle**: Separation of concerns - recorder writes, backend handles queries.

---

## 🎯 Goals

1. **Configuration File Support**: YAML/JSON5 config for all settings
2. **Configurable Flush Triggers**: User-defined size and time thresholds
3. **Multi-Backend Support**: ReductStore, InfluxDB, File System, S3, etc.
4. **Per-Topic Compression**: Optimize compression based on data type
5. **Backward Compatibility**: Existing code continues to work

---

## 🏗️ Architecture Changes

### Current Architecture (ReductStore Only)

```
Zenoh Subscribers → Topic Buffers → Flush Workers → ReductStore Client → ReductStore
```

### Proposed Architecture (Multi-Backend, Write-Only Agent)

```
┌─────────────────────────────────────────────────────────────┐
│ Recorder Agent (Write-Only)                                 │
│                                                              │
│ Zenoh Subscribers → Topic Buffers → Flush Workers           │
│                                            ↓                 │
│                                    StorageBackend Trait      │
│                                            ↓                 │
└────────────────────────────────────────────┬────────────────┘
                                             │
                                             ↓
                    ┌────────────────────────────────────┐
                    │    Backend (Direct Write)          │
                    │                                    │
                    │  - ReductStore (time-series)       │
                    │  - InfluxDB (metrics)              │
                    │  - Filesystem (MCAP files)         │
                    │  - S3 (cloud archive)              │
                    └────────────────────────────────────┘
                                             ↓
                    ┌────────────────────────────────────┐
                    │  Query Layer (Separate)            │
                    │                                    │
                    │  Users query backend directly:     │
                    │  - ReductStore Web UI              │
                    │  - Grafana dashboards              │
                    │  - MCAP tools / Foxglove           │
                    │  - S3 Select / Athena              │
                    └────────────────────────────────────┘
```

**Key Design Points**:
1. **Recorder = Write-Only**: No query support needed in recorder
2. **Direct Backend Write**: Minimal latency, no middleware
3. **Backend-Native Queries**: Use specialized tools for each backend
4. **Lightweight Agent**: Can run on resource-constrained devices

---

## 📝 Configuration File Design

### Option 1: YAML Configuration (Recommended)

**File**: `config.yaml`

```yaml
# Zenoh Recorder Configuration

# Zenoh connection settings
zenoh:
  mode: peer  # peer, client, or router
  connect:
    endpoints:
      - tcp/localhost:7447
      - udp/224.0.0.1:7447
  listen:
    endpoints:
      - tcp/0.0.0.0:17447  # Optional listen port
  scouting:
    multicast:
      enabled: true
      address: 224.0.0.1:7447

# Storage backend configuration
storage:
  # Backend type: reductstore, influxdb, filesystem, s3, zenoh-plugin
  backend: reductstore
  
  # Backend-specific settings
  reductstore:
    url: http://localhost:8383
    bucket_name: zenoh_recordings
    api_token: ${REDUCT_API_TOKEN}  # Support env vars
    timeout_seconds: 300
    max_retries: 3
    connection_pool:
      max_idle: 10
      idle_timeout_seconds: 90
      tcp_keepalive_seconds: 60
  
  # Alternative: InfluxDB backend
  influxdb:
    url: http://localhost:8086
    org: my-org
    bucket: zenoh-data
    token: ${INFLUX_TOKEN}
  
  # Alternative: Filesystem backend
  filesystem:
    base_path: /data/recordings
    file_format: mcap  # mcap, parquet, json
    rotation:
      max_size_mb: 1024  # Rotate after 1GB
      max_duration_seconds: 3600  # Rotate after 1 hour
  
  # Alternative: S3/Object Storage
  s3:
    endpoint: https://s3.amazonaws.com
    region: us-west-2
    bucket: zenoh-recordings
    access_key_id: ${AWS_ACCESS_KEY_ID}
    secret_access_key: ${AWS_SECRET_ACCESS_KEY}
    prefix: recordings/  # Optional key prefix

# Recording settings
recorder:
  device_id: ${DEVICE_ID:-robot-001}  # Default value
  
  # Buffer flush policies
  flush_policy:
    # Size-based trigger
    max_buffer_size_bytes: 10485760  # 10 MB
    
    # Time-based trigger
    max_buffer_duration_seconds: 10  # 10 seconds
    
    # Minimum samples before flush (avoid tiny flushes)
    min_samples_per_flush: 10
  
  # Compression settings
  compression:
    default_type: zstd  # none, lz4, zstd
    default_level: 2    # 0-4 (fastest to slowest)
    
    # Per-topic compression overrides
    per_topic:
      "/camera/**":
        type: lz4
        level: 1  # Fast compression for high-frequency camera
      "/lidar/**":
        type: zstd
        level: 3  # Better compression for lidar
  
  # Worker thread pool
  workers:
    flush_workers: 4      # Concurrent flush operations
    upload_workers: 2     # Concurrent uploads
    queue_capacity: 1000  # Max pending flush tasks
  
  # Control interface
  control:
    key_prefix: recorder/control  # Queryable key prefix
    status_key: recorder/status/**
    timeout_seconds: 30

# Logging configuration
logging:
  level: info  # trace, debug, info, warn, error
  format: json  # text, json
  output: stdout  # stdout, stderr, file
  file_path: /var/log/zenoh-recorder.log

# Metrics/Monitoring
metrics:
  enabled: true
  prometheus:
    enabled: true
    port: 9090
    path: /metrics
  
  # Optional: Export to other systems
  otlp:
    enabled: false
    endpoint: http://localhost:4317

# Advanced settings
advanced:
  # Enable local spooling if backend unavailable
  local_spool:
    enabled: true
    directory: /tmp/zenoh-recorder-spool
    max_size_gb: 10
  
  # Memory limits
  memory:
    max_buffer_memory_mb: 512
    warn_threshold_percent: 80
```

### Option 2: JSON5 Configuration (More Flexible)

**File**: `config.json5`

```json5
{
  // Zenoh configuration
  zenoh: {
    mode: "peer",
    connect: {
      endpoints: ["tcp/localhost:7447"]
    }
  },
  
  // Storage backend
  storage: {
    backend: "reductstore",
    config: {
      url: "http://localhost:8383",
      bucket_name: "zenoh_data",
      max_retries: 3
    }
  },
  
  // Recorder settings
  recorder: {
    device_id: "${DEVICE_ID}",
    flush_policy: {
      max_buffer_size_bytes: 10485760,
      max_buffer_duration_seconds: 10
    },
    compression: {
      default_type: "zstd",
      default_level: 2
    }
  }
}
```

---

## 🔌 Storage Backend Abstraction

### Trait-Based Backend System

```rust
/// Generic storage backend trait
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Initialize the backend (create bucket/database if needed)
    async fn initialize(&self) -> Result<()>;
    
    /// Write a record with metadata
    async fn write_record(
        &self,
        entry_name: &str,
        timestamp_us: u64,
        data: Vec<u8>,
        labels: HashMap<String, String>,
    ) -> Result<()>;
    
    /// Write with retry logic
    async fn write_with_retry(
        &self,
        entry_name: &str,
        timestamp_us: u64,
        data: Vec<u8>,
        labels: HashMap<String, String>,
        max_retries: u32,
    ) -> Result<()>;
    
    /// Health check
    async fn health_check(&self) -> Result<bool>;
    
    /// Get backend info
    fn backend_type(&self) -> &str;
}
```

### Implementation for Multiple Backends

```rust
// 1. ReductStore Backend (existing)
pub struct ReductStoreBackend {
    client: ReductStoreClient,
    config: ReductStoreConfig,
}

#[async_trait]
impl StorageBackend for ReductStoreBackend {
    async fn initialize(&self) -> Result<()> {
        self.client.ensure_bucket().await
    }
    
    async fn write_record(/* ... */) -> Result<()> {
        self.client.write_record(/* ... */).await
    }
    
    fn backend_type(&self) -> &str { "reductstore" }
}

// 2. Zenoh Storage Manager Backend (NEW!)
pub struct ZenohStorageBackend {
    session: Arc<Session>,
    storage_key_prefix: String,
}

#[async_trait]
impl StorageBackend for ZenohStorageBackend {
    async fn initialize(&self) -> Result<()> {
        // Zenoh storage manager handles initialization
        Ok(())
    }
    
    async fn write_record(
        &self,
        entry_name: &str,
        timestamp_us: u64,
        data: Vec<u8>,
        labels: HashMap<String, String>,
    ) -> Result<()> {
        // Write to Zenoh storage using key expression
        let key = format!("{}/{}/{}", 
            self.storage_key_prefix, entry_name, timestamp_us);
        
        // Attach labels as attachment
        let mut put_builder = self.session.put(&key, data);
        
        // Encode labels as attachment (Zenoh supports attachments)
        let labels_json = serde_json::to_vec(&labels)?;
        put_builder = put_builder.with_attachment(labels_json);
        
        put_builder.res().await?;
        Ok(())
    }
    
    fn backend_type(&self) -> &str { "zenoh-storage" }
}

// 3. Filesystem Backend
pub struct FilesystemBackend {
    base_path: PathBuf,
    file_format: FileFormat,
}

#[async_trait]
impl StorageBackend for FilesystemBackend {
    async fn write_record(
        &self,
        entry_name: &str,
        timestamp_us: u64,
        data: Vec<u8>,
        labels: HashMap<String, String>,
    ) -> Result<()> {
        let file_path = self.base_path
            .join(entry_name)
            .join(format!("{}.mcap", timestamp_us));
        
        tokio::fs::create_dir_all(file_path.parent().unwrap()).await?;
        
        // Write MCAP file with labels as metadata
        let mut file = tokio::fs::File::create(&file_path).await?;
        file.write_all(&data).await?;
        
        // Write labels as sidecar JSON
        let labels_path = file_path.with_extension("json");
        let labels_json = serde_json::to_vec_pretty(&labels)?;
        tokio::fs::write(labels_path, labels_json).await?;
        
        Ok(())
    }
    
    fn backend_type(&self) -> &str { "filesystem" }
}

// 4. InfluxDB Backend
pub struct InfluxDbBackend {
    client: influxdb::Client,
    bucket: String,
}

#[async_trait]
impl StorageBackend for InfluxDbBackend {
    async fn write_record(
        &self,
        entry_name: &str,
        timestamp_us: u64,
        data: Vec<u8>,
        labels: HashMap<String, String>,
    ) -> Result<()> {
        use influxdb::{Timestamp, WriteQuery};
        
        let mut query = WriteQuery::new(
            Timestamp::Microseconds(timestamp_us as i64),
            entry_name,
        );
        
        // Add labels as tags
        for (key, value) in labels {
            query = query.add_tag(key, value);
        }
        
        // Add data as base64 field
        let data_b64 = base64::encode(data);
        query = query.add_field("data", data_b64);
        
        self.client.query(&query).await?;
        Ok(())
    }
    
    fn backend_type(&self) -> &str { "influxdb" }
}
```

---

## 🔧 Alternative: Zenoh Storage Manager (Option A - Not Recommended for Write-Only)

**Note**: This approach is included for completeness but is **NOT recommended** for write-only recording agents.

### Why Not Recommended for This Use Case

- ❌ **Over-engineered**: Query features not needed (users query backend directly)
- ❌ **Added complexity**: Middleware layer not beneficial for writes
- ❌ **Less control**: Can't optimize for backend-specific features

### When Zenoh Storage Manager Makes Sense

Use Zenoh Storage Manager if you need:
- ✅ Query historical data **through Zenoh** (not just at backend)
- ✅ Built-in replication across multiple Zenoh nodes
- ✅ Consistent Zenoh-based API for reads and writes

### Example Implementation (For Reference Only)

```rust
// This is NOT the recommended approach for write-only agents!
pub struct ZenohStorageBackend {
    session: Arc<Session>,
    storage_key_prefix: String,
}

#[async_trait]
impl StorageBackend for ZenohStorageBackend {
    async fn write_record(
        &self,
        entry_name: &str,
        timestamp_us: u64,
        data: Vec<u8>,
        labels: HashMap<String, String>,
    ) -> Result<()> {
        // Write to Zenoh storage using key expression
        let key = format!("{}/{}/{}", 
            self.storage_key_prefix, entry_name, timestamp_us);
        
        // Attach labels as attachment
        let labels_json = serde_json::to_vec(&labels)?;
        self.session
            .put(&key, data)
            .with_attachment(labels_json)
            .res()
            .await?;
        Ok(())
    }
    
    fn backend_type(&self) -> &str { "zenoh-storage" }
}
```

**For write-only agents, use Option B (direct backend writes) instead.**

---

## 📐 Proposed File Structure

```
zenoh-recorder/
├── config/
│   ├── default.yaml          # Default configuration
│   ├── production.yaml       # Production settings
│   ├── development.yaml      # Dev settings
│   └── examples/
│       ├── reductstore.yaml  # ReductStore-specific config
│       ├── influxdb.yaml     # InfluxDB example
│       ├── filesystem.yaml   # File system example
│       └── zenoh-storage.yaml # Zenoh storage manager example
│
├── src/
│   ├── config/
│   │   ├── mod.rs            # Config loading and validation
│   │   ├── types.rs          # Config structs
│   │   └── validation.rs     # Config validation logic
│   │
│   ├── storage/
│   │   ├── mod.rs            # Storage backend trait export
│   │   ├── backend.rs        # StorageBackend trait definition
│   │   ├── reductstore.rs    # ReductStore implementation (primary)
│   │   ├── filesystem.rs     # File system backend (optional)
│   │   ├── influxdb.rs       # InfluxDB backend (optional)
│   │   ├── s3.rs             # S3 backend (optional)
│   │   └── factory.rs        # Backend factory pattern
│   │
│   ├── recorder.rs           # Updated to use StorageBackend trait
│   ├── buffer.rs             # Updated to use config flush policy
│   └── ...
```

---

## 💻 Implementation Design

### 1. Configuration Structures

```rust
// src/config/types.rs

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Main configuration structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecorderConfig {
    pub zenoh: ZenohConfig,
    pub storage: StorageConfig,
    pub recorder: RecorderSettings,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub metrics: MetricsConfig,
    #[serde(default)]
    pub advanced: AdvancedConfig,
}

/// Zenoh configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ZenohConfig {
    #[serde(default = "default_mode")]
    pub mode: String,  // "peer", "client", "router"
    
    pub connect: Option<ConnectConfig>,
    pub listen: Option<ListenConfig>,
    
    #[serde(default)]
    pub scouting: ScoutingConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnectConfig {
    pub endpoints: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListenConfig {
    pub endpoints: Vec<String>,
}

/// Storage configuration with backend selection
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageConfig {
    /// Backend type: reductstore, influxdb, filesystem, s3, zenoh-storage
    pub backend: String,
    
    #[serde(flatten)]
    pub backend_config: BackendConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BackendConfig {
    ReductStore(ReductStoreConfig),
    ZenohStorage(ZenohStorageConfig),
    Filesystem(FilesystemConfig),
    InfluxDb(InfluxDbConfig),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReductStoreConfig {
    pub url: String,
    pub bucket_name: String,
    pub api_token: Option<String>,
    
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    
    #[serde(default = "default_retries")]
    pub max_retries: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ZenohStorageConfig {
    /// Storage key prefix (e.g., "recordings")
    pub key_prefix: String,
    
    /// Storage volume configuration
    pub volume: VolumeConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VolumeConfig {
    pub id: String,
    pub backend: String,  // "memory", "rocksdb", "influxdb"
    pub url: Option<String>,
}

/// Recorder-specific settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecorderSettings {
    pub device_id: String,
    pub flush_policy: FlushPolicy,
    pub compression: CompressionConfig,
    pub workers: WorkerConfig,
    pub control: ControlConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FlushPolicy {
    /// Maximum buffer size in bytes before flush
    pub max_buffer_size_bytes: usize,
    
    /// Maximum duration in seconds before flush
    pub max_buffer_duration_seconds: u64,
    
    /// Minimum samples before flush (avoid tiny flushes)
    #[serde(default = "default_min_samples")]
    pub min_samples_per_flush: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompressionConfig {
    pub default_type: String,  // "none", "lz4", "zstd"
    pub default_level: u8,     // 0-4
    
    #[serde(default)]
    pub per_topic: HashMap<String, TopicCompression>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TopicCompression {
    pub r#type: String,
    pub level: u8,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkerConfig {
    #[serde(default = "default_flush_workers")]
    pub flush_workers: usize,
    
    #[serde(default = "default_upload_workers")]
    pub upload_workers: usize,
    
    #[serde(default = "default_queue_capacity")]
    pub queue_capacity: usize,
}

// Default value functions
fn default_mode() -> String { "peer".to_string() }
fn default_timeout() -> u64 { 300 }
fn default_retries() -> u32 { 3 }
fn default_min_samples() -> usize { 10 }
fn default_flush_workers() -> usize { 4 }
fn default_upload_workers() -> usize { 2 }
fn default_queue_capacity() -> usize { 1000 }
```

### 2. Configuration Loading

```rust
// src/config/mod.rs

use anyhow::{Context, Result};
use std::path::Path;

pub struct ConfigLoader;

impl ConfigLoader {
    /// Load configuration from file with environment variable substitution
    pub fn load<P: AsRef<Path>>(path: P) -> Result<RecorderConfig> {
        let content = std::fs::read_to_string(path.as_ref())
            .context("Failed to read config file")?;
        
        // Substitute environment variables
        let content = Self::substitute_env_vars(&content);
        
        // Parse based on file extension
        let config = if path.as_ref().extension().unwrap() == "yaml" {
            serde_yaml::from_str(&content)?
        } else if path.as_ref().extension().unwrap() == "json5" {
            json5::from_str(&content)?
        } else {
            bail!("Unsupported config format");
        };
        
        // Validate configuration
        Self::validate(&config)?;
        
        Ok(config)
    }
    
    /// Load from multiple sources with precedence
    pub fn load_with_overrides(
        default: Option<&str>,
        env_file: Option<&str>,
        overrides: HashMap<String, String>,
    ) -> Result<RecorderConfig> {
        // 1. Start with default
        let mut config = if let Some(path) = default {
            Self::load(path)?
        } else {
            RecorderConfig::default()
        };
        
        // 2. Override with environment-specific file
        if let Some(env_path) = env_file {
            let env_config = Self::load(env_path)?;
            config = Self::merge(config, env_config);
        }
        
        // 3. Apply CLI overrides
        for (key, value) in overrides {
            Self::apply_override(&mut config, &key, &value)?;
        }
        
        Ok(config)
    }
    
    /// Substitute ${VAR} and ${VAR:-default} patterns
    fn substitute_env_vars(content: &str) -> String {
        let re = regex::Regex::new(r"\$\{([^}:]+)(?::-([^}]+))?\}").unwrap();
        
        re.replace_all(content, |caps: &regex::Captures| {
            let var_name = &caps[1];
            let default_value = caps.get(2).map(|m| m.as_str());
            
            std::env::var(var_name)
                .or_else(|_| default_value.map(|s| s.to_string()).ok_or(()))
                .unwrap_or_else(|_| format!("${{{}}}", var_name))
        }).to_string()
    }
    
    fn validate(config: &RecorderConfig) -> Result<()> {
        // Validate flush policy
        if config.recorder.flush_policy.max_buffer_size_bytes == 0 {
            bail!("max_buffer_size_bytes must be > 0");
        }
        
        if config.recorder.flush_policy.max_buffer_duration_seconds == 0 {
            bail!("max_buffer_duration_seconds must be > 0");
        }
        
        // Validate compression level
        if config.recorder.compression.default_level > 4 {
            bail!("compression level must be 0-4");
        }
        
        // Validate backend-specific settings
        match config.storage.backend.as_str() {
            "reductstore" => {
                // Validate ReductStore config
            }
            "zenoh-storage" => {
                // Validate Zenoh storage config
            }
            _ => bail!("Unknown backend: {}", config.storage.backend),
        }
        
        Ok(())
    }
}
```

### 3. Backend Factory

```rust
// src/storage/factory.rs

pub struct BackendFactory;

impl BackendFactory {
    /// Create storage backend from configuration
    pub fn create(
        config: &StorageConfig,
        session: Arc<Session>,
    ) -> Result<Arc<dyn StorageBackend>> {
        match config.backend.as_str() {
            "reductstore" => {
                let backend_config = config.get_reductstore_config()?;
                let backend = ReductStoreBackend::new(backend_config)?;
                Ok(Arc::new(backend))
            }
            
            "zenoh-storage" => {
                let backend_config = config.get_zenoh_storage_config()?;
                let backend = ZenohStorageBackend::new(session, backend_config)?;
                Ok(Arc::new(backend))
            }
            
            "filesystem" => {
                let backend_config = config.get_filesystem_config()?;
                let backend = FilesystemBackend::new(backend_config)?;
                Ok(Arc::new(backend))
            }
            
            "influxdb" => {
                let backend_config = config.get_influxdb_config()?;
                let backend = InfluxDbBackend::new(backend_config)?;
                Ok(Arc::new(backend))
            }
            
            unknown => bail!("Unknown storage backend: {}", unknown),
        }
    }
}
```

### 4. Updated RecorderManager

```rust
// src/recorder.rs (updated)

pub struct RecorderManager {
    session: Arc<Session>,
    sessions: Arc<DashMap<String, Arc<RecordingSession>>>,
    
    // Generic storage backend (not tied to ReductStore!)
    storage_backend: Arc<dyn StorageBackend>,
    
    flush_queue: Arc<ArrayQueue<FlushTask>>,
    
    // Configuration
    config: RecorderConfig,
}

impl RecorderManager {
    pub fn new(
        session: Arc<Session>,
        config: RecorderConfig,
    ) -> Result<Self> {
        // Create storage backend from config
        let storage_backend = BackendFactory::create(&config.storage, session.clone())?;
        
        let flush_queue = Arc::new(ArrayQueue::new(
            config.recorder.workers.queue_capacity
        ));
        
        let manager = Self {
            session,
            sessions: Arc::new(DashMap::new()),
            storage_backend,
            flush_queue: flush_queue.clone(),
            config,
        };
        
        // Start workers with configured count
        manager.start_flush_workers();
        
        Ok(manager)
    }
    
    fn start_flush_workers(&self) {
        // Use configured worker count
        for i in 0..self.config.recorder.workers.flush_workers {
            let flush_queue = self.flush_queue.clone();
            let storage_backend = self.storage_backend.clone();
            let sessions = self.sessions.clone();
            let compression_config = self.config.recorder.compression.clone();
            
            tokio::spawn(async move {
                debug!("Flush worker {} started", i);
                loop {
                    if let Some(task) = flush_queue.pop() {
                        Self::process_flush_task(
                            task,
                            storage_backend.clone(),
                            sessions.clone(),
                            &compression_config,
                        ).await;
                    } else {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            });
        }
    }
    
    // Create buffer with configured flush policy
    fn create_topic_buffer(&self, topic: &str, recording_id: &str) -> Arc<TopicBuffer> {
        let flush_policy = &self.config.recorder.flush_policy;
        
        Arc::new(TopicBuffer::new(
            topic.to_string(),
            recording_id.to_string(),
            flush_policy.max_buffer_size_bytes,
            Duration::from_secs(flush_policy.max_buffer_duration_seconds),
            self.flush_queue.clone(),
        ))
    }
}
```

---

## 🔄 Implementation Plan (Option B - Trait-Based)

### Phase 1: Configuration System (Day 1-2)
- ✅ Add YAML config file structures
- ✅ Implement config loader with env var substitution (`${VAR}` and `${VAR:-default}`)
- ✅ Add config validation
- ✅ Update CLI to accept `--config` parameter
- ✅ Keep ReductStore as only backend (backward compatible)
- ✅ Add configurable flush triggers (size/time)
- ✅ Add per-topic compression settings

### Phase 2: Storage Backend Abstraction (Day 3-4)
- ✅ Define `StorageBackend` trait
- ✅ Refactor ReductStore to implement trait
- ✅ Update RecorderManager to use trait
- ✅ Add backend factory pattern
- ✅ Update tests for trait-based system

### Phase 3 (Optional): Additional Backends
- ⚪ Filesystem backend (MCAP files to disk)
- ⚪ InfluxDB backend (if users need metrics)
- ⚪ S3 backend (for cloud archival)
- ⚪ Multi-backend writes (primary + fallback)

---

## 📋 Example Configurations

### Example 1: ReductStore (Current Default)

```yaml
# config/reductstore.yaml

zenoh:
  mode: peer
  connect:
    endpoints:
      - tcp/localhost:7447

storage:
  backend: reductstore
  reductstore:
    url: http://localhost:8383
    bucket_name: zenoh_recordings
    max_retries: 3

recorder:
  device_id: robot-001
  flush_policy:
    max_buffer_size_bytes: 10485760  # 10 MB
    max_buffer_duration_seconds: 10
  compression:
    default_type: zstd
    default_level: 2
  workers:
    flush_workers: 4
```

### Example 2: InfluxDB Backend (Metrics & Analytics)

```yaml
# config/influxdb.yaml

zenoh:
  mode: peer
  connect:
    endpoints:
      - tcp/localhost:7447

storage:
  backend: influxdb
  influxdb:
    url: http://localhost:8086
    org: robotics-team
    bucket: sensor-data
    token: ${INFLUX_TOKEN}
    timeout_seconds: 60

recorder:
  device_id: robot-001
  flush_policy:
    max_buffer_size_bytes: 5242880   # 5 MB (smaller for metrics)
    max_buffer_duration_seconds: 5   # Faster flush for real-time
  compression:
    default_type: lz4  # Fast compression for metrics
    default_level: 1
  workers:
    flush_workers: 2  # Fewer workers for metrics
```

### Example 3: Filesystem Backend

```yaml
# config/filesystem.yaml

zenoh:
  mode: peer

storage:
  backend: filesystem
  filesystem:
    base_path: /data/recordings
    file_format: mcap
    rotation:
      max_size_mb: 1024
      max_duration_seconds: 3600

recorder:
  device_id: robot-001
  flush_policy:
    max_buffer_size_bytes: 10485760
    max_buffer_duration_seconds: 10
  compression:
    default_type: none  # Already in MCAP format
    default_level: 0
```

---

## 🚀 Usage Examples

### CLI with Config File

```bash
# Use specific config file
zenoh-recorder --config config/production.yaml

# Override specific values
zenoh-recorder \
  --config config/default.yaml \
  --device-id robot-042 \
  --flush-size 20971520

# Environment-based config selection
ENV=production zenoh-recorder --config config/${ENV}.yaml
```

### Programmatic Usage

```rust
use zenoh_recorder::config::ConfigLoader;
use zenoh_recorder::recorder::RecorderManager;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = ConfigLoader::load("config/production.yaml")?;
    
    // Create Zenoh session from config
    let session = zenoh::open(config.zenoh.clone()).res().await?;
    
    // Create recorder with config
    let recorder = RecorderManager::new(Arc::new(session), config)?;
    
    // Use recorder...
    Ok(())
}
```

---

## ✅ Benefits of This Design

| Feature | Benefit |
|---------|---------|
| **YAML/JSON5 Config** | Easy to read, edit, version control |
| **Env Var Substitution** | Secure secrets management |
| **Multi-Backend** | Choose best backend for use case |
| **Zenoh Storage** | Leverage built-in infrastructure |
| **Trait-Based** | Easy to add new backends |
| **Validation** | Catch config errors early |
| **Backward Compatible** | Existing code still works |
| **Per-Topic Settings** | Optimize compression per data type |

---

## 🎯 Recommended Approach

### ⭐ **Option B: Trait-Based Multi-Backend** (RECOMMENDED)

**Why Option B is ideal for write-only recording agents**:

The recorder is a **write-only agent** - it doesn't need query features!
- Recorder writes data to backend
- Users query backend directly (ReductStore UI, Grafana, etc.)
- Simpler, focused design

**Pros**:
- ✅ Full control over each backend
- ✅ Direct integration (no middleware overhead)
- ✅ Lightweight agent (minimal dependencies)
- ✅ Backend-specific optimizations
- ✅ Easy to add new backends via trait
- ✅ Users leverage specialized query tools at backend

**Cons**:
- ⚠️ More code to maintain (but focused on writes only)
- ⚠️ Each backend needs implementation

**Perfect for**: Write-only recording agents on devices

---

### Option A: **Zenoh Storage Manager**

**Pros**:
- ✅ Uses Zenoh's built-in storage infrastructure
- ✅ Query support through Zenoh (read historical data)
- ✅ Built-in replication

**Cons**:
- ⚠️ Adds complexity for write-only use case
- ⚠️ Query features not needed (users query backend directly)
- ⚠️ Learning curve for Zenoh storage API

**Use when**: You need to query data through Zenoh (not applicable here)

---

### Option C: **Hybrid Approach**

```yaml
storage:
  backends:
    - type: reductstore
      primary: true
        
    - type: filesystem
      fallback: true  # Local backup if network fails
```

**Benefits**:
- ✅ Primary + fallback for reliability
- ✅ Write to multiple backends for redundancy

**Use when**: Need redundancy or gradual migration between backends

---

## 📊 Comparison: Current vs Proposed

| Aspect | Current | Proposed |
|--------|---------|----------|
| **Configuration** | Env vars only | YAML/JSON5 files |
| **Backend** | ReductStore only | Multiple backends |
| **Flush Triggers** | Hardcoded (10MB/10s) | Configurable |
| **Compression** | Per-recording | Per-topic override |
| **Workers** | Hardcoded (4) | Configurable |
| **Zenoh Setup** | Hardcoded | Full config support |
| **Flexibility** | Low | High |
| **Complexity** | Low | Medium |

---

## 🤔 Configuration Decisions

Based on the **write-only agent** use case, here are the recommended choices:

### 1. **Approach**: Option B (Trait-Based Multi-Backend) ✅ RECOMMENDED

**Why**:
- Recorder is write-only agent → doesn't need Zenoh Storage Manager's query features
- Direct backend writes → minimal latency
- Users query backend directly → use specialized tools (ReductStore UI, Grafana, etc.)
- Lightweight and focused

### 2. **Config Format**: YAML ✅ RECOMMENDED

**Why**:
- Most readable for operations teams
- Industry standard for K8s, Docker Compose
- Good environment variable support
- Easy to validate

### 3. **Implementation Scope**

**⭐ Recommended: Minimal Scope First** (1-2 days)
```
✅ Configuration file support (YAML)
✅ Configurable flush triggers (size/time)
✅ Per-topic compression settings
✅ StorageBackend trait abstraction
✅ Keep ReductStore as primary backend
⚪ (Future) Add filesystem backend for offline scenarios
⚪ (Future) Add InfluxDB/S3 if needed
```

**Medium Scope** (3-4 days)
```
All of Minimal +
✅ Filesystem backend (MCAP files)
✅ Comprehensive config validation
✅ Config schema documentation
```

**Full Scope** (1 week)
```
All of Medium +
✅ InfluxDB backend
✅ S3 backend
✅ Multi-backend writes (primary + fallback)
```

### 4. **Priority Backends**

1. **ReductStore** (existing, keep it) - Best for time-series data
2. **Filesystem** (add next) - Good for offline/edge scenarios
3. **InfluxDB** (optional) - If users need metrics/analytics
4. **S3** (optional) - For cloud archival

---

## ✋ Awaiting Your Approval

**Ready to proceed with:**
- ✅ **Approach**: Option B (Trait-Based Multi-Backend)
- ✅ **Config Format**: YAML
- ✅ **Scope**: Minimal (config + trait + ReductStore)
- ✅ **Optional**: Filesystem backend for phase 2

**May I proceed with implementation?** 🚀

Please confirm or let me know if you'd like any adjustments!

---

## 📝 Summary

**Design Decision**: Option B (Trait-Based Multi-Backend) - Write-Only Agent

**Rationale**:
- Recorder is a **write-only agent** focused on efficient data ingestion
- Users **query backends directly** using specialized tools (ReductStore UI, Grafana, MCAP viewers)
- Direct backend writes provide **minimal latency** and **full control**
- Lightweight design suitable for **resource-constrained devices**

**Implementation Approach**:
1. Add YAML configuration system with environment variable support
2. Make flush triggers configurable (size/time)
3. Support per-topic compression settings
4. Abstract storage via `StorageBackend` trait
5. Keep ReductStore as primary backend
6. Allow easy addition of future backends (filesystem, InfluxDB, S3)

**Expected Timeline**: 2-4 days for minimal scope

---

**Document Version**: 2.0  
**Last Updated**: October 18, 2025  
**Status**: ✅ **DESIGN APPROVED - READY FOR IMPLEMENTATION**

