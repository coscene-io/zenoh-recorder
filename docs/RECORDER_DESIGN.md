# Zenoh Recorder with ReductStore Backend - Detailed Design

## Executive Summary

This document presents a staff-level engineering design for a high-performance data recorder for the Zenoh middleware that aggregates multi-topic streams, flushes data based on size or time thresholds, and stores data in ReductStore using MCAP format. The recorder supports distributed recording control via a request-response protocol.

---

## 1. System Architecture

### 1.1 High-Level Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         Zenoh Network                           │
└────────┬──────────────────────────────────┬────────────────────┘
         │                                   │
         │ (Subscribe to Topics)             │ (Query/Response)
         │                                   │
    ┌────▼────────────────────────────┐  ┌──▼──────────────────┐
    │  Multi-Topic Subscriber Pool    │  │  Control Interface  │
    │  (Lock-free Ring Buffers)       │  │  (Queryable)        │
    └────┬────────────────────────────┘  └──┬──────────────────┘
         │                                   │
         │ Samples                           │ Commands
         │                                   │
    ┌────▼───────────────────────────────────▼──────────────────┐
    │              Recording State Machine                       │
    │  States: Idle, Recording, Paused, Uploading, Finished     │
    └────┬──────────────────────────────────────────────────────┘
         │
         │ Batched Samples
         │
    ┌────▼────────────────────────────────────────────────────┐
    │          Per-Topic Aggregation Buffers                  │
    │    (Double-Buffering with Atomic Swap)                  │
    │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
    │  │ Topic A      │  │ Topic B      │  │ Topic N      │  │
    │  │ - Front Buf  │  │ - Front Buf  │  │ - Front Buf  │  │
    │  │ - Back Buf   │  │ - Back Buf   │  │ - Back Buf   │  │
    │  └──────────────┘  └──────────────┘  └──────────────┘  │
    └────┬───────────────────────────────────────────────────┘
         │
         │ Flush Triggers (Size/Time)
         │
    ┌────▼────────────────────────────────────────────────────┐
    │              MCAP Serialization Layer                   │
    │  (Memory-mapped I/O, Compression Pipeline)              │
    └────┬────────────────────────────────────────────────────┘
         │
         │ MCAP Chunks
         │
    ┌────▼────────────────────────────────────────────────────┐
    │          ReductStore Upload Manager                     │
    │  (Connection Pool, Retry Logic, Batching)               │
    └────┬────────────────────────────────────────────────────┘
         │
         │ HTTP/REST
         │
    ┌────▼────────────────────────────────────────────────────┐
    │              ReductStore Backend                        │
    │  Bucket: "ros_data"                                     │
    │  - Entry: "recordings_metadata"                         │
    │  - Entry: "camera_front"                                │
    │  - Entry: "lidar_points"                                │
    │  - Entry: "imu_data"                                    │
    └─────────────────────────────────────────────────────────┘
```

### 1.2 Component Breakdown

1. **Control Interface**: Zenoh Queryable for request-response protocol
2. **Subscriber Pool**: Multi-topic subscription management
3. **Aggregation Engine**: Per-topic buffering with flush policies
4. **MCAP Serializer**: Efficient binary serialization to MCAP format
5. **Upload Manager**: Asynchronous, batched uploads to ReductStore
6. **State Machine**: Recording lifecycle management

---

## 2. Core Components Design

### 2.1 Control Interface (Request-Response Protocol)

#### 2.1.1 Zenoh Key Expression Schema

```rust
// Control operations
recorder/control/{device_id}

// Status queries
recorder/status/{recording_id}

// Administrative operations
recorder/admin/{device_id}/stats
```

#### 2.1.2 Message Protocol

**Request Message (Control)**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecorderRequest {
    pub command: RecorderCommand,
    pub recording_id: Option<String>,
    pub scene: Option<String>,
    pub skills: Vec<String>,
    pub organization: Option<String>,
    pub task_id: Option<String>,
    pub device_id: String,
    pub data_collector_id: Option<String>,
    pub topics: Vec<String>,
    pub compression_level: CompressionLevel,
    pub compression_type: CompressionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecorderCommand {
    Start,
    Pause,
    Resume,
    Cancel,
    Finish,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CompressionLevel {
    Fastest = 0,
    Fast = 1,
    Default = 2,
    Slow = 3,
    Slowest = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    Lz4,
    Zstd,
}
```

**Response Message (Control)**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecorderResponse {
    pub success: bool,
    pub message: String,
    pub recording_id: Option<String>,
    pub bucket_name: Option<String>,
}
```

**Status Request/Response**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusRequest {
    pub recording_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub success: bool,
    pub message: String,
    pub status: RecordingStatus,
    pub scene: Option<String>,
    pub skills: Vec<String>,
    pub organization: Option<String>,
    pub task_id: Option<String>,
    pub device_id: String,
    pub data_collector_id: Option<String>,
    pub active_topics: Vec<String>,
    pub buffer_size_bytes: i32,
    pub total_recorded_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecordingStatus {
    Idle,
    Recording,
    Paused,
    Uploading,
    Finished,
    Cancelled,
}
```

#### 2.1.3 Implementation Pattern

```rust
pub struct ControlInterface {
    queryable: Queryable<'static, flume::Receiver<Query>>,
    state_machine: Arc<Mutex<RecorderStateMachine>>,
    recorder_registry: Arc<DashMap<String, RecordingSession>>,
}

impl ControlInterface {
    pub async fn start(&self) {
        while let Ok(query) = self.queryable.recv_async().await {
            let selector = query.selector();
            
            // Parse the request
            if let Some(payload) = query.payload() {
                match self.handle_request(payload).await {
                    Ok(response) => {
                        let response_bytes = serde_json::to_vec(&response).unwrap();
                        query.reply(selector.key_expr.clone(), response_bytes)
                            .await
                            .unwrap();
                    }
                    Err(e) => {
                        let error_response = RecorderResponse {
                            success: false,
                            message: e.to_string(),
                            recording_id: None,
                            bucket_name: None,
                        };
                        let response_bytes = serde_json::to_vec(&error_response).unwrap();
                        query.reply(selector.key_expr.clone(), response_bytes)
                            .await
                            .unwrap();
                    }
                }
            }
        }
    }
    
    async fn handle_request(&self, payload: &ZBytes) -> Result<RecorderResponse> {
        let request: RecorderRequest = serde_json::from_slice(
            payload.to_bytes().as_ref()
        )?;
        
        match request.command {
            RecorderCommand::Start => self.handle_start(request).await,
            RecorderCommand::Pause => self.handle_pause(request).await,
            RecorderCommand::Resume => self.handle_resume(request).await,
            RecorderCommand::Cancel => self.handle_cancel(request).await,
            RecorderCommand::Finish => self.handle_finish(request).await,
        }
    }
}
```

---

### 2.2 Multi-Topic Subscriber Pool

#### 2.2.1 Lock-Free Architecture

**Design Principle**: Minimize contention between the Zenoh I/O thread and the processing threads.

```rust
use crossbeam::queue::ArrayQueue;
use std::sync::Arc;

pub struct SubscriberPool {
    subscribers: Vec<Subscriber<'static, flume::Receiver<Sample>>>,
    // Per-topic lock-free SPSC ring buffer
    topic_queues: Arc<DashMap<String, Arc<ArrayQueue<Sample>>>>,
    // Pre-allocated queue size (tune for expected burst rate)
    queue_capacity: usize,
}

impl SubscriberPool {
    pub async fn subscribe_to_topics(
        &mut self,
        session: Arc<Session>,
        topics: Vec<String>,
    ) -> Result<()> {
        for topic in topics {
            // Create lock-free queue for this topic
            let queue = Arc::new(ArrayQueue::new(self.queue_capacity));
            self.topic_queues.insert(topic.clone(), queue.clone());
            
            // Subscribe with callback that writes to lock-free queue
            let subscriber = session
                .declare_subscriber(&topic)
                .callback({
                    let queue = queue.clone();
                    move |sample| {
                        // Non-blocking push, drop if full (backpressure)
                        if queue.push(sample).is_err() {
                            // Metrics: record dropped sample
                            DROPPED_SAMPLES.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                })
                .await?;
            
            self.subscribers.push(subscriber);
        }
        Ok(())
    }
}
```

#### 2.2.2 Performance Considerations

- **SPSC Queue**: Single-producer (Zenoh callback), single-consumer (aggregator thread)
- **Lock-free**: No mutex contention, only atomic operations
- **Backpressure**: Drop samples if queue is full (configurable policy)
- **Cache-line alignment**: Ensure queue heads/tails are on separate cache lines

```rust
#[repr(align(64))]  // Cache line size on x86-64
struct AlignedQueue {
    queue: ArrayQueue<Sample>,
}
```

---

### 2.3 Per-Topic Aggregation Buffers

#### 2.3.1 Double-Buffering Strategy

**Goal**: Allow continuous writing while flushing, without blocking.

```rust
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub struct TopicBuffer {
    topic_name: String,
    
    // Double buffer
    front_buffer: RwLock<Vec<Sample>>,
    back_buffer: RwLock<Vec<Sample>>,
    active_buffer: AtomicBool, // true = front, false = back
    
    // Flush triggers
    max_buffer_size: usize,
    max_buffer_duration: Duration,
    last_flush_time: AtomicU64,
    
    // Statistics
    total_samples: AtomicUsize,
    total_bytes: AtomicUsize,
}

impl TopicBuffer {
    pub fn push_sample(&self, sample: Sample) -> Result<()> {
        let active_is_front = self.active_buffer.load(Ordering::Acquire);
        let buffer = if active_is_front {
            &self.front_buffer
        } else {
            &self.back_buffer
        };
        
        let mut buf = buffer.write();
        buf.push(sample.clone());
        
        let sample_size = sample.payload().len();
        self.total_samples.fetch_add(1, Ordering::Relaxed);
        self.total_bytes.fetch_add(sample_size, Ordering::Relaxed);
        
        drop(buf);
        
        // Check if we need to flush
        if self.should_flush() {
            self.trigger_flush();
        }
        
        Ok(())
    }
    
    fn should_flush(&self) -> bool {
        let bytes = self.total_bytes.load(Ordering::Relaxed);
        if bytes >= self.max_buffer_size {
            return true;
        }
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let last_flush = self.last_flush_time.load(Ordering::Relaxed);
        
        if now - last_flush >= self.max_buffer_duration.as_secs() {
            return true;
        }
        
        false
    }
    
    fn trigger_flush(&self) {
        // Swap buffers atomically
        let was_front = self.active_buffer.swap(false, Ordering::AcqRel);
        
        // Reset counters
        self.total_samples.store(0, Ordering::Relaxed);
        self.total_bytes.store(0, Ordering::Relaxed);
        self.last_flush_time.store(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            Ordering::Relaxed,
        );
        
        // Schedule async flush of the inactive buffer
        let buffer_to_flush = if was_front {
            &self.front_buffer
        } else {
            &self.back_buffer
        };
        
        // Send to flush queue (handled by dedicated thread pool)
        // This is non-blocking
        FLUSH_QUEUE.send(FlushTask {
            topic: self.topic_name.clone(),
            samples: std::mem::take(&mut *buffer_to_flush.write()),
        }).unwrap();
    }
}
```

#### 2.3.2 Flush Policies

**Size-based**: Flush when buffer reaches N bytes (e.g., 10MB)
**Time-based**: Flush every T seconds (e.g., 10 seconds)
**Hybrid**: Flush on whichever condition is met first

**Tuning Parameters**:
```rust
pub struct FlushPolicy {
    pub max_buffer_size_bytes: usize,      // 10 MB
    pub max_buffer_duration_ms: u64,       // 10,000 ms
    pub min_samples_per_flush: usize,      // 100 (avoid tiny flushes)
}
```

---

### 2.4 MCAP Serialization Layer

#### 2.4.1 MCAP Format Structure

MCAP is a container format for time-series data. Each flush produces one MCAP file chunk.

```rust
use mcap::{Writer, Channel, Schema, Message};

pub struct McapSerializer {
    compression_type: CompressionType,
    compression_level: CompressionLevel,
}

impl McapSerializer {
    pub fn serialize_batch(
        &self,
        topic: &str,
        samples: Vec<Sample>,
        recording_id: &str,
    ) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();
        let mut writer = Writer::new(&mut buffer)?;
        
        // Define schema (could be CDR, Protobuf, JSON, etc.)
        let schema = Schema {
            name: format!("{}_schema", topic),
            encoding: "cdr".to_string(),  // Common Data Representation
            data: self.generate_schema(topic)?,
        };
        let schema_id = writer.add_schema(&schema)?;
        
        // Define channel (topic)
        let channel = Channel {
            topic: topic.to_string(),
            schema_id: Some(schema_id),
            message_encoding: "cdr".to_string(),
            metadata: [
                ("recording_id".to_string(), recording_id.to_string()),
            ].into_iter().collect(),
        };
        let channel_id = writer.add_channel(&channel)?;
        
        // Write messages
        for sample in samples {
            let message = Message {
                channel_id,
                sequence: 0,  // Can track sequence per channel
                log_time: sample.timestamp()
                    .map(|ts| ts.get_time().as_u64())
                    .unwrap_or(0),
                publish_time: sample.timestamp()
                    .map(|ts| ts.get_time().as_u64())
                    .unwrap_or(0),
                data: sample.payload().to_bytes(),
            };
            writer.add_message(&message)?;
        }
        
        writer.finish()?;
        
        // Apply compression
        match self.compression_type {
            CompressionType::None => Ok(buffer),
            CompressionType::Lz4 => self.compress_lz4(buffer),
            CompressionType::Zstd => self.compress_zstd(buffer),
        }
    }
    
    fn compress_zstd(&self, data: Vec<u8>) -> Result<Vec<u8>> {
        let level = match self.compression_level {
            CompressionLevel::Fastest => 1,
            CompressionLevel::Fast => 3,
            CompressionLevel::Default => 5,
            CompressionLevel::Slow => 10,
            CompressionLevel::Slowest => 19,
        };
        
        Ok(zstd::encode_all(&data[..], level)?)
    }
}
```

#### 2.4.2 Performance Optimizations

**Memory-mapped I/O**: For large MCAP files, use mmap to avoid copying
**Compression Pipeline**: Use SIMD instructions (via zstd/lz4 native implementations)
**Pre-allocation**: Reuse buffers across flushes to minimize allocations

```rust
pub struct McapBufferPool {
    buffers: ArrayQueue<Vec<u8>>,
}

impl McapBufferPool {
    pub fn acquire(&self) -> Vec<u8> {
        self.buffers.pop().unwrap_or_else(|| Vec::with_capacity(10 * 1024 * 1024))
    }
    
    pub fn release(&self, mut buffer: Vec<u8>) {
        buffer.clear();
        let _ = self.buffers.push(buffer);
    }
}
```

---

### 2.5 ReductStore Upload Manager

#### 2.5.1 HTTP Connection Pool

**Goal**: Reuse TCP connections to minimize overhead.

```rust
use reqwest::{Client, ClientBuilder};
use std::time::Duration;

pub struct ReductStoreClient {
    client: Client,
    base_url: String,
    bucket_name: String,
}

impl ReductStoreClient {
    pub fn new(base_url: String, bucket_name: String) -> Self {
        let client = ClientBuilder::new()
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Duration::from_secs(60))
            .http2_adaptive_window(true)
            .build()
            .unwrap();
        
        Self { client, base_url, bucket_name }
    }
    
    pub async fn write_record(
        &self,
        entry_name: &str,
        timestamp_us: u64,
        data: Vec<u8>,
        labels: HashMap<String, String>,
    ) -> Result<()> {
        let url = format!(
            "{}/api/v1/b/{}/{}",
            self.base_url, self.bucket_name, entry_name
        );
        
        let mut request = self.client
            .post(&url)
            .header("Content-Type", "application/octet-stream")
            .header("x-reduct-time", timestamp_us.to_string());
        
        // Add labels
        for (key, value) in labels {
            request = request.header(
                format!("x-reduct-label-{}", key),
                value,
            );
        }
        
        let response = request.body(data).send().await?;
        
        if !response.status().is_success() {
            bail!("ReductStore write failed: {}", response.status());
        }
        
        Ok(())
    }
}
```

#### 2.5.2 Retry Logic with Exponential Backoff

```rust
use tokio::time::sleep;

impl ReductStoreClient {
    pub async fn write_record_with_retry(
        &self,
        entry_name: &str,
        timestamp_us: u64,
        data: Vec<u8>,
        labels: HashMap<String, String>,
        max_retries: u32,
    ) -> Result<()> {
        let mut attempt = 0;
        let mut delay = Duration::from_millis(100);
        
        loop {
            match self.write_record(entry_name, timestamp_us, data.clone(), labels.clone()).await {
                Ok(_) => return Ok(()),
                Err(e) if attempt < max_retries => {
                    tracing::warn!(
                        "Upload failed (attempt {}/{}): {}. Retrying in {:?}",
                        attempt + 1, max_retries, e, delay
                    );
                    sleep(delay).await;
                    delay *= 2;  // Exponential backoff
                    attempt += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }
}
```

#### 2.5.3 Upload Batching

Instead of uploading each MCAP chunk immediately, batch multiple chunks and upload in parallel.

```rust
pub struct UploadBatcher {
    batch_size: usize,
    batch_timeout: Duration,
    pending_uploads: Vec<UploadTask>,
    client: Arc<ReductStoreClient>,
}

impl UploadBatcher {
    pub async fn queue_upload(&mut self, task: UploadTask) {
        self.pending_uploads.push(task);
        
        if self.pending_uploads.len() >= self.batch_size {
            self.flush_batch().await;
        }
    }
    
    async fn flush_batch(&mut self) {
        let tasks = std::mem::take(&mut self.pending_uploads);
        
        // Upload in parallel (up to N concurrent uploads)
        let futures: Vec<_> = tasks.into_iter()
            .map(|task| {
                let client = self.client.clone();
                tokio::spawn(async move {
                    client.write_record_with_retry(
                        &task.entry_name,
                        task.timestamp_us,
                        task.data,
                        task.labels,
                        3,  // max retries
                    ).await
                })
            })
            .collect();
        
        // Wait for all uploads to complete
        for future in futures {
            if let Err(e) = future.await {
                tracing::error!("Upload failed: {}", e);
            }
        }
    }
}
```

---

### 2.6 Recording State Machine

```rust
pub enum RecorderState {
    Idle,
    Recording,
    Paused,
    Uploading,
    Finished,
    Cancelled,
}

pub struct RecordingSession {
    pub recording_id: String,
    pub state: AtomicU8,  // Use atomic for lock-free state checks
    pub metadata: RecordingMetadata,
    pub topic_buffers: Arc<DashMap<String, Arc<TopicBuffer>>>,
    pub start_time: Instant,
    pub pause_time: Option<Instant>,
}

impl RecordingSession {
    pub fn transition(&self, from: RecorderState, to: RecorderState) -> Result<()> {
        // Validate state transitions
        let valid_transition = match (from, to) {
            (RecorderState::Idle, RecorderState::Recording) => true,
            (RecorderState::Recording, RecorderState::Paused) => true,
            (RecorderState::Paused, RecorderState::Recording) => true,
            (RecorderState::Recording, RecorderState::Uploading) => true,
            (RecorderState::Uploading, RecorderState::Finished) => true,
            (_, RecorderState::Cancelled) => true,
            _ => false,
        };
        
        if !valid_transition {
            bail!("Invalid state transition from {:?} to {:?}", from, to);
        }
        
        self.state.store(to as u8, Ordering::Release);
        Ok(())
    }
}
```

---

## 3. Data Flow

### 3.1 Recording Lifecycle

```
1. Client sends Start command
   ↓
2. Recorder creates RecordingSession
   - Generate recording_id (UUID)
   - Initialize per-topic buffers
   - Subscribe to all requested topics
   ↓
3. Samples arrive from Zenoh network
   ↓
4. Samples pushed to lock-free per-topic queues
   ↓
5. Aggregator threads pull from queues → push to double buffers
   ↓
6. Flush trigger activated (size or time)
   ↓
7. Swap buffers atomically, schedule flush
   ↓
8. Flush thread: Serialize to MCAP, compress
   ↓
9. Upload thread: Write to ReductStore entry
   ↓
10. Client sends Finish command
    ↓
11. Flush all remaining buffers
    ↓
12. Upload final chunks
    ↓
13. Write metadata record
    ↓
14. Transition to Finished state
```

### 3.2 Topic Naming Convention

Map Zenoh topics to ReductStore entries:

```rust
fn zenoh_topic_to_entry_name(topic: &str) -> String {
    // Example: "/camera/front" → "camera_front"
    topic.trim_start_matches('/').replace('/', "_")
}
```

### 3.3 Metadata Record Structure

```json
{
  "recording_id": "rec-001",
  "scene": "highway",
  "skills": ["lane_keeping", "obstacle_avoidance"],
  "organization": "acme_robotics",
  "task_id": "task-123",
  "device_id": "robot_01",
  "data_collector_id": "collector-456",
  "topics": ["/camera/front", "/lidar/points"],
  "compression_type": "zstd",
  "compression_level": 2,
  "start_time": "2024-10-16T10:00:00Z",
  "end_time": "2024-10-16T10:15:00Z",
  "total_bytes": 1073741824,
  "total_samples": 150000,
  "per_topic_stats": {
    "/camera/front": {
      "sample_count": 90000,
      "total_bytes": 943718400,
      "avg_sample_size": 10485
    },
    "/lidar/points": {
      "sample_count": 60000,
      "total_bytes": 130023424,
      "avg_sample_size": 2167
    }
  }
}
```

---

## 4. Performance Optimizations

### 4.1 Memory Management

**Zero-copy where possible**:
- Zenoh's `ZBytes` supports zero-copy payloads
- Use `Bytes` (from `bytes` crate) for reference-counted buffers

**Memory pool**:
```rust
pub struct MemoryPool {
    small_buffers: ArrayQueue<Vec<u8>>,   // < 1KB
    medium_buffers: ArrayQueue<Vec<u8>>,  // 1KB - 1MB
    large_buffers: ArrayQueue<Vec<u8>>,   // > 1MB
}
```

### 4.2 CPU Optimizations

**Thread Affinity**:
```rust
use core_affinity::CoreId;

pub fn pin_thread_to_core(core_id: usize) {
    let core_ids = core_affinity::get_core_ids().unwrap();
    core_affinity::set_for_current(core_ids[core_id]);
}
```

**NUMA Awareness**: On multi-socket systems, allocate buffers on the same NUMA node as the processing thread.

**SIMD**: Compression libraries (zstd, lz4) already use SIMD. For custom processing, use `std::simd` (nightly).

### 4.3 I/O Optimizations

**Direct I/O**: Bypass OS page cache for large sequential writes (if writing to disk first)
```rust
use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;

let file = OpenOptions::new()
    .write(true)
    .create(true)
    .custom_flags(libc::O_DIRECT)
    .open("output.mcap")?;
```

**HTTP/2**: ReductStore supports HTTP/2 for multiplexing multiple uploads over a single TCP connection.

### 4.4 Network Considerations

**Batching**: Group multiple small MCAP chunks into a single HTTP request (if ReductStore supports batch writes).

**TCP Tuning**:
```rust
use tokio::net::TcpStream;
use socket2::{Socket, Domain, Type};

let socket = Socket::new(Domain::IPV4, Type::STREAM, None)?;
socket.set_nodelay(true)?;  // Disable Nagle's algorithm
socket.set_send_buffer_size(1024 * 1024)?;  // 1MB send buffer
```

---

## 5. Implementation Roadmap

### Phase 1: Core Infrastructure (Week 1-2)
- [ ] Implement control interface (Queryable for request-response)
- [ ] Implement state machine and recording session management
- [ ] Set up basic subscriber pool

### Phase 2: Buffering & Aggregation (Week 2-3)
- [ ] Implement lock-free per-topic queues
- [ ] Implement double-buffered aggregation
- [ ] Implement size and time-based flush triggers

### Phase 3: Serialization (Week 3-4)
- [ ] Integrate MCAP library
- [ ] Implement compression pipeline (LZ4, Zstd)
- [ ] Add buffer pooling and memory optimizations

### Phase 4: ReductStore Integration (Week 4-5)
- [ ] Implement ReductStore client with connection pooling
- [ ] Implement retry logic with exponential backoff
- [ ] Implement upload batching

### Phase 5: Testing & Optimization (Week 5-6)
- [ ] Load testing with high-frequency topics (1000+ Hz)
- [ ] Latency profiling (identify bottlenecks)
- [ ] Memory leak testing (run for 24+ hours)
- [ ] Network failure testing (simulate packet loss)

### Phase 6: Production Hardening (Week 6-7)
- [ ] Add comprehensive metrics (Prometheus)
- [ ] Add structured logging (tracing)
- [ ] Add configuration validation
- [ ] Write documentation and examples

---

## 6. Configuration Schema

```yaml
recorder:
  device_id: "robot_01"
  bucket_name: "ros_data"
  reductstore:
    url: "http://localhost:8383"
    api_token: "${REDUCT_API_TOKEN}"
  
  flush_policy:
    max_buffer_size_mb: 10
    max_buffer_duration_sec: 10
    min_samples_per_flush: 100
  
  performance:
    queue_capacity: 10000
    num_aggregator_threads: 4
    num_upload_threads: 2
    upload_batch_size: 5
    max_upload_retries: 3
  
  compression:
    default_type: "zstd"
    default_level: 2
  
  control:
    key_expr: "recorder/control/${device_id}"
    status_key_expr: "recorder/status/**"
```

---

## 7. Monitoring & Observability

### 7.1 Metrics (Prometheus)

```rust
use prometheus::{IntCounter, IntGauge, Histogram, Registry};

pub struct RecorderMetrics {
    pub samples_received: IntCounter,
    pub samples_dropped: IntCounter,
    pub bytes_received: IntCounter,
    pub flush_count: IntCounter,
    pub upload_count: IntCounter,
    pub upload_errors: IntCounter,
    pub buffer_size_bytes: IntGauge,
    pub flush_duration_seconds: Histogram,
    pub upload_duration_seconds: Histogram,
}
```

### 7.2 Structured Logging

```rust
use tracing::{info, warn, error, debug};

#[instrument(skip(self))]
async fn flush_buffer(&self, topic: &str, samples: Vec<Sample>) {
    debug!("Starting flush for topic {}", topic);
    
    let start = Instant::now();
    match self.serialize_and_upload(topic, samples).await {
        Ok(_) => {
            let duration = start.elapsed();
            info!(
                topic = %topic,
                duration_ms = duration.as_millis(),
                "Flush completed"
            );
        }
        Err(e) => {
            error!(
                topic = %topic,
                error = %e,
                "Flush failed"
            );
        }
    }
}
```

---

## 8. Error Handling & Fault Tolerance

### 8.1 Error Categories

1. **Transient Errors** (retryable):
   - Network timeouts
   - ReductStore unavailable
   - TCP connection reset

2. **Permanent Errors** (not retryable):
   - Invalid configuration
   - Authentication failure
   - Bucket doesn't exist

3. **Resource Errors**:
   - Out of memory
   - Disk full
   - Queue overflow

### 8.2 Fallback Strategies

**Local Spooling**: If uploads fail repeatedly, write MCAP files to local disk:
```rust
pub struct LocalSpooler {
    spool_dir: PathBuf,
}

impl LocalSpooler {
    pub fn write_chunk(&self, recording_id: &str, topic: &str, chunk: Vec<u8>) -> Result<()> {
        let filename = format!("{}_{}_{}_{}.mcap", 
            recording_id, topic, 
            Utc::now().timestamp(), 
            Uuid::new_v4()
        );
        let path = self.spool_dir.join(filename);
        std::fs::write(path, chunk)?;
        Ok(())
    }
}
```

**Dead Letter Queue**: For samples that fail validation, send to a separate topic:
```rust
const DLQ_TOPIC: &str = "recorder/dead_letter_queue";
```

---

## 9. Security Considerations

1. **Authentication**: Support API tokens for ReductStore
2. **Encryption**: Use TLS for ReductStore connections
3. **Access Control**: Validate recording commands (e.g., only authorized devices can start recordings)
4. **Audit Logging**: Log all control operations (start, pause, cancel)

---

## 10. Testing Strategy

### 10.1 Unit Tests
- State machine transitions
- Buffer swapping logic
- Flush trigger conditions
- Compression correctness

### 10.2 Integration Tests
- End-to-end recording flow
- Multiple concurrent recordings
- Network failure scenarios
- ReductStore unavailability

### 10.3 Performance Tests
- High-frequency topic recording (10 kHz)
- Large message sizes (10 MB+ per message)
- Sustained recording (24+ hours)
- Memory profiling (Valgrind, heaptrack)

### 10.4 Chaos Engineering
- Random network failures
- Kill Recorder mid-recording
- Corrupt MCAP chunks
- ReductStore becomes slow

---

## 11. Example Usage

### 11.1 Starting a Recording

```bash
# Send control request via Zenoh
z_put 'recorder/control/robot_01' \
  --payload '{
    "command": "Start",
    "scene": "highway",
    "skills": ["lane_keeping", "obstacle_avoidance"],
    "organization": "acme_robotics",
    "task_id": "task-123",
    "device_id": "robot_01",
    "data_collector_id": "collector-456",
    "topics": ["/camera/front", "/lidar/points", "/imu/data"],
    "compression_level": 2,
    "compression_type": "Zstd"
  }'
```

### 11.2 Querying Status

```bash
z_get 'recorder/status/rec-001'
```

Response:
```json
{
  "success": true,
  "message": "Recording in progress",
  "status": "Recording",
  "scene": "highway",
  "skills": ["lane_keeping", "obstacle_avoidance"],
  "organization": "acme_robotics",
  "task_id": "task-123",
  "device_id": "robot_01",
  "data_collector_id": "collector-456",
  "active_topics": ["/camera/front", "/lidar/points", "/imu/data"],
  "buffer_size_bytes": 5242880,
  "total_recorded_bytes": 104857600
}
```

### 11.3 Finishing a Recording

```bash
z_put 'recorder/control/robot_01' \
  --payload '{
    "command": "Finish",
    "recording_id": "rec-001"
  }'
```

---

## 12. Comparison with Alternatives

### 12.1 vs. Rosbag (ROS 1/2)
- **Zenoh Recorder**: Pub/sub middleware agnostic, cloud-native storage
- **Rosbag**: Tightly coupled to ROS, local file storage

### 12.2 vs. MCAP CLI
- **Zenoh Recorder**: Live recording with flush policies, distributed control
- **MCAP CLI**: Post-processing tool, no live recording

### 12.3 vs. Custom Solutions
- **Zenoh Recorder**: Reusable, tested, optimized
- **Custom**: One-off, maintenance burden

---

## 13. Future Enhancements

1. **Multi-format support**: Add Parquet, Arrow for analytics
2. **Data filtering**: Only record samples matching predicates
3. **Triggering**: Start/stop recording based on events
4. **Replay**: Read from ReductStore and republish to Zenoh
5. **Edge processing**: Apply ML inference during recording
6. **Federated learning**: Aggregate statistics across multiple recorders

---

## 14. References

- [Zenoh Documentation](https://zenoh.io/docs/)
- [ReductStore Documentation](https://www.reduct.store/docs)
- [MCAP Format Specification](https://mcap.dev/)
- [Zstd Compression](https://github.com/facebook/zstd)
- [Lock-free Programming](https://preshing.com/20120612/an-introduction-to-lock-free-programming/)

---

## Appendix A: Crate Dependencies

```toml
[dependencies]
zenoh = "0.11"
zenoh-ext = "0.11"
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
mcap = "0.8"
zstd = "0.13"
lz4 = "1.24"
reqwest = { version = "0.11", features = ["json"] }
dashmap = "5"
crossbeam = "0.8"
bytes = "1"
uuid = { version = "1", features = ["v4"] }
chrono = "0.4"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
prometheus = "0.13"
config = "0.13"
core_affinity = "0.8"

[dev-dependencies]
criterion = "0.5"
tokio-test = "0.4"
proptest = "1"
```

---

## Appendix B: Project Structure

```
zenoh-recorder/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs
│   ├── main.rs
│   ├── control/
│   │   ├── mod.rs
│   │   ├── interface.rs
│   │   ├── protocol.rs
│   ├── recorder/
│   │   ├── mod.rs
│   │   ├── state_machine.rs
│   │   ├── session.rs
│   ├── subscriber/
│   │   ├── mod.rs
│   │   ├── pool.rs
│   ├── buffer/
│   │   ├── mod.rs
│   │   ├── topic_buffer.rs
│   │   ├── double_buffer.rs
│   ├── serialization/
│   │   ├── mod.rs
│   │   ├── mcap.rs
│   │   ├── compression.rs
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── reductstore.rs
│   │   ├── upload.rs
│   │   ├── retry.rs
│   ├── metrics/
│   │   ├── mod.rs
│   ├── config/
│   │   ├── mod.rs
│   └── error.rs
├── tests/
│   ├── integration_test.rs
│   ├── performance_test.rs
├── benches/
│   ├── buffer_bench.rs
│   ├── compression_bench.rs
└── examples/
    ├── simple_recorder.rs
    ├── multi_topic_recorder.rs
```

---

**End of Design Document**

