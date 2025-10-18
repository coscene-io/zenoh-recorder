use anyhow::Result;
use crossbeam::queue::ArrayQueue;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, warn};
use zenoh::prelude::Buffer;
use zenoh::sample::Sample;

/// Message to flush buffer
#[derive(Clone)]
pub struct FlushTask {
    pub topic: String,
    pub samples: Vec<Sample>,
    pub recording_id: String,
}

/// Double-buffered topic buffer with flush policies
pub struct TopicBuffer {
    topic_name: String,
    recording_id: String,

    // Double buffer
    front_buffer: Arc<RwLock<Vec<Sample>>>,
    back_buffer: Arc<RwLock<Vec<Sample>>>,
    active_is_front: AtomicBool, // true = front is active, false = back is active

    // Flush triggers
    max_buffer_size: usize,
    max_buffer_duration: Duration,
    last_flush_time: AtomicU64,

    // Statistics
    total_samples: AtomicUsize,
    total_bytes: AtomicUsize,

    // Flush queue
    flush_queue: Arc<ArrayQueue<FlushTask>>,
}

impl TopicBuffer {
    pub fn new(
        topic_name: String,
        recording_id: String,
        max_buffer_size: usize,
        max_buffer_duration: Duration,
        flush_queue: Arc<ArrayQueue<FlushTask>>,
    ) -> Self {
        Self {
            topic_name,
            recording_id,
            front_buffer: Arc::new(RwLock::new(Vec::new())),
            back_buffer: Arc::new(RwLock::new(Vec::new())),
            active_is_front: AtomicBool::new(true),
            max_buffer_size,
            max_buffer_duration,
            last_flush_time: AtomicU64::new(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            ),
            total_samples: AtomicUsize::new(0),
            total_bytes: AtomicUsize::new(0),
            flush_queue,
        }
    }

    /// Push a sample to the active buffer
    pub async fn push_sample(&self, sample: Sample) -> Result<()> {
        let active_is_front = self.active_is_front.load(Ordering::Acquire);
        let buffer = if active_is_front {
            &self.front_buffer
        } else {
            &self.back_buffer
        };

        let sample_size = sample.payload.len();

        {
            let mut buf = buffer.write().await;
            buf.push(sample);
        }

        self.total_samples.fetch_add(1, Ordering::Relaxed);
        self.total_bytes.fetch_add(sample_size, Ordering::Relaxed);

        // Check if we need to flush
        if self.should_flush() {
            self.trigger_flush().await;
        }

        Ok(())
    }

    /// Check if buffer should be flushed
    fn should_flush(&self) -> bool {
        let bytes = self.total_bytes.load(Ordering::Relaxed);
        if bytes >= self.max_buffer_size {
            debug!(
                "Buffer size threshold reached for topic '{}': {} bytes",
                self.topic_name, bytes
            );
            return true;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let last_flush = self.last_flush_time.load(Ordering::Relaxed);

        if now - last_flush >= self.max_buffer_duration.as_secs() {
            debug!(
                "Time threshold reached for topic '{}': {} seconds",
                self.topic_name,
                now - last_flush
            );
            return true;
        }

        false
    }

    /// Trigger buffer flush
    async fn trigger_flush(&self) {
        // Swap buffers atomically
        let was_front = self.active_is_front.fetch_xor(true, Ordering::AcqRel);

        // Get the buffer to flush (the one that was active)
        let buffer_to_flush = if was_front {
            &self.front_buffer
        } else {
            &self.back_buffer
        };

        // Extract samples
        let samples = {
            let mut buf = buffer_to_flush.write().await;
            std::mem::take(&mut *buf)
        };

        let sample_count = samples.len();
        let bytes = samples.iter().map(|s| s.payload.len()).sum::<usize>();

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

        debug!(
            "Flushing {} samples ({} bytes) from topic '{}'",
            sample_count, bytes, self.topic_name
        );

        // Send to flush queue
        let task = FlushTask {
            topic: self.topic_name.clone(),
            samples,
            recording_id: self.recording_id.clone(),
        };

        if self.flush_queue.push(task).is_err() {
            warn!(
                "Flush queue full for topic '{}', dropping flush task",
                self.topic_name
            );
        }
    }

    /// Force flush remaining data
    pub async fn force_flush(&self) -> Result<()> {
        self.trigger_flush().await;
        Ok(())
    }

    /// Get statistics
    pub fn stats(&self) -> (usize, usize) {
        (
            self.total_samples.load(Ordering::Relaxed),
            self.total_bytes.load(Ordering::Relaxed),
        )
    }
}
