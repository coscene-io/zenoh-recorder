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

use crossbeam::queue::ArrayQueue;
use std::sync::Arc;
use std::time::Duration;
use zenoh::key_expr::KeyExpr;
use zenoh::sample::Sample;
use zenoh_recorder::buffer::{FlushTask, TopicBuffer};

fn create_sample(topic: &'static str, data: Vec<u8>) -> Sample {
    let key: KeyExpr<'static> = topic.try_into().unwrap();
    Sample::new(key, data)
}

#[tokio::test]
async fn test_topic_buffer_creation() {
    let flush_queue = Arc::new(ArrayQueue::new(10));
    let buffer = TopicBuffer::new(
        "/test/topic".to_string(),
        "rec-123".to_string(),
        1024 * 1024, // 1 MB
        Duration::from_secs(10),
        flush_queue,
    );

    let (samples, bytes) = buffer.stats();
    assert_eq!(samples, 0);
    assert_eq!(bytes, 0);
}

#[tokio::test]
async fn test_topic_buffer_push_sample() {
    let flush_queue = Arc::new(ArrayQueue::new(10));
    let buffer = TopicBuffer::new(
        "/test/topic".to_string(),
        "rec-123".to_string(),
        1024 * 1024,
        Duration::from_secs(10),
        flush_queue,
    );

    let sample = create_sample("test/topic", b"test payload".to_vec());
    buffer.push_sample(sample).await.unwrap();

    let (samples, bytes) = buffer.stats();
    assert_eq!(samples, 1);
    assert!(bytes > 0);
}

#[tokio::test]
async fn test_topic_buffer_size_trigger() {
    let flush_queue = Arc::new(ArrayQueue::new(10));
    let buffer = TopicBuffer::new(
        "/test/topic".to_string(),
        "rec-123".to_string(),
        100, // Small buffer (100 bytes)
        Duration::from_secs(10),
        flush_queue.clone(),
    );

    // Push enough samples to trigger size-based flush
    for i in 0..10 {
        let sample = create_sample("test/topic", format!("payload_{}", i).into_bytes());
        buffer.push_sample(sample).await.unwrap();
    }

    // Give it a moment to process
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Should have triggered at least one flush
    assert!(flush_queue.len() > 0 || buffer.stats().1 < 100);
}

#[tokio::test]
async fn test_topic_buffer_force_flush() {
    let flush_queue = Arc::new(ArrayQueue::new(10));
    let buffer = TopicBuffer::new(
        "/test/topic".to_string(),
        "rec-123".to_string(),
        1024 * 1024,
        Duration::from_secs(10),
        flush_queue.clone(),
    );

    // Push some samples
    for i in 0..5 {
        let sample = create_sample("test/topic", format!("data_{}", i).into_bytes());
        buffer.push_sample(sample).await.unwrap();
    }

    // Force flush
    buffer.force_flush().await.unwrap();

    // Give it a moment
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Stats should be reset
    let (samples, bytes) = buffer.stats();
    assert_eq!(samples, 0);
    assert_eq!(bytes, 0);
}

#[test]
fn test_flush_task_creation() {
    let samples = vec![];
    let task = FlushTask {
        topic: "/test".to_string(),
        samples,
        recording_id: "rec-001".to_string(),
    };

    assert_eq!(task.topic, "/test");
    assert_eq!(task.recording_id, "rec-001");
    assert_eq!(task.samples.len(), 0);
}

#[tokio::test]
async fn test_buffer_stats_accuracy() {
    let flush_queue = Arc::new(ArrayQueue::new(10));
    let buffer = TopicBuffer::new(
        "/test/topic".to_string(),
        "rec-123".to_string(),
        10 * 1024 * 1024,
        Duration::from_secs(10),
        flush_queue,
    );

    // Push multiple samples
    let test_data = b"test data with some length";
    for _ in 0..10 {
        let sample = create_sample("test/topic", test_data.to_vec());
        buffer.push_sample(sample).await.unwrap();
    }

    let (samples, bytes) = buffer.stats();
    assert_eq!(samples, 10);
    assert!(bytes >= test_data.len() * 10);
}

#[tokio::test]
async fn test_multiple_pushes() {
    let flush_queue = Arc::new(ArrayQueue::new(10));
    let buffer = TopicBuffer::new(
        "/test/topic".to_string(),
        "rec-123".to_string(),
        10 * 1024 * 1024,
        Duration::from_secs(10),
        flush_queue,
    );

    // Push samples in batches
    for batch in 0..3 {
        for i in 0..10 {
            let data = format!("batch_{}_sample_{}", batch, i);
            let sample = create_sample("test/topic", data.into_bytes());
            buffer.push_sample(sample).await.unwrap();
        }
    }

    let (samples, bytes) = buffer.stats();
    assert_eq!(samples, 30);
    assert!(bytes > 0);
}

#[tokio::test]
async fn test_concurrent_pushes() {
    let flush_queue = Arc::new(ArrayQueue::new(100));
    let buffer = Arc::new(TopicBuffer::new(
        "/test/topic".to_string(),
        "rec-123".to_string(),
        10 * 1024 * 1024,
        Duration::from_secs(10),
        flush_queue,
    ));

    // Spawn multiple tasks pushing samples
    let mut handles = vec![];
    for task_id in 0..5 {
        let buffer_clone = buffer.clone();
        let handle = tokio::spawn(async move {
            for i in 0..10 {
                let data = format!("task_{}_sample_{}", task_id, i);
                let sample = create_sample("test/topic", data.into_bytes());
                buffer_clone.push_sample(sample).await.unwrap();
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    let (samples, _bytes) = buffer.stats();
    assert_eq!(samples, 50); // 5 tasks * 10 samples
}

