use anyhow::Result;
use std::time::Duration;
use zenoh::prelude::r#async::*;

/// Simple test client for publishing data that can be recorded
#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting test data publisher...");

    let session = zenoh::open(Config::default())
        .res()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    println!("Zenoh session opened");
    println!("\nPublishing test data to /test/topic1 and /test/topic2");
    println!("Start the recorder in another terminal to capture this data\n");

    for i in 0..100 {
        session
            .put("/test/topic1", format!("test_data_topic1_{}", i))
            .res()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        session
            .put("/test/topic2", format!("test_data_topic2_{}", i))
            .res()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        if i % 10 == 0 {
            println!("Published {} samples", i);
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    println!("\n✓ Published 100 samples to each topic");
    println!("Note: To test the recorder, use zenoh_recorder with control commands");
    println!("Example: Use curl or z_get to send control messages to recorder/control/robot_01");

    Ok(())
}
