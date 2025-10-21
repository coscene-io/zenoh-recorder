// Example: How to use custom proto definitions with zenoh-recorder
//
// This example demonstrates how users can provide their own protobuf serialization
// and have the recorder store it as-is (schema-agnostic approach).

use anyhow::Result;
use zenoh::prelude::r#async::*;

// Example: User defines their own proto message
// (In real usage, this would be generated from user's .proto files)
#[derive(Clone, PartialEq, prost::Message)]
pub struct MyCustomMessage {
    #[prost(string, tag = "1")]
    pub sensor_id: String,
    #[prost(double, tag = "2")]
    pub temperature: f64,
    #[prost(double, tag = "3")]
    pub humidity: f64,
    #[prost(int64, tag = "4")]
    pub timestamp_ms: i64,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 1. User creates their own proto message
    let my_data = MyCustomMessage {
        sensor_id: "DHT22-001".to_string(),
        temperature: 23.5,
        humidity: 65.2,
        timestamp_ms: chrono::Utc::now().timestamp_millis(),
    };

    // 2. User serializes it themselves (using prost)
    let buffer = prost::Message::encode_to_vec(&my_data);

    // 3. Publish to Zenoh (recorder stores raw bytes)
    let session = zenoh::open(Config::default())
        .res()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to open Zenoh session: {}", e))?;

    session
        .put("/sensors/temperature", buffer)
        .res()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to publish message: {}", e))?;

    println!("✅ Published custom proto message to Zenoh");
    println!("   Recorder will store it as-is (raw bytes)");
    println!("   Schema metadata can be configured in recorder config:");
    println!();
    println!("   schema:");
    println!("     per_topic:");
    println!("       \"/sensors/temperature\":");
    println!("         format: protobuf");
    println!("         schema_name: my_package.MyCustomMessage");
    println!();
    println!("📦 Later, user deserializes:");
    println!("   let data = storage.get(...);");
    println!("   let msg = MyCustomMessage::decode(data.payload)?;");

    Ok(())
}

// Summary of approach:
//
// ✅ User handles their own serialization
// ✅ Recorder is format-agnostic (just stores bytes)
// ✅ Schema metadata is optional (for documentation)
// ✅ No recompilation of recorder needed
// ✅ Works with any serialization format (proto, JSON, msgpack, etc.)
