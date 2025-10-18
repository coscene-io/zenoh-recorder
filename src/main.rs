use anyhow::Result;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use zenoh::prelude::r#async::*;

mod buffer;
mod control;
mod mcap_writer;
mod protocol;
mod recorder;
mod storage;

use control::ControlInterface;
use recorder::RecorderManager;

// Include protobuf definitions
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/sensor_data.rs"));
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Zenoh Recorder");

    // Configuration from environment variables
    let device_id = std::env::var("DEVICE_ID").unwrap_or_else(|_| "robot_01".to_string());
    let reductstore_url =
        std::env::var("REDUCTSTORE_URL").unwrap_or_else(|_| "http://localhost:8383".to_string());
    let bucket_name = std::env::var("BUCKET_NAME").unwrap_or_else(|_| "ros_data".to_string());

    // Open Zenoh session
    let config = Config::default();
    let session = Arc::new(
        zenoh::open(config)
            .res()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?,
    );

    info!("Zenoh session opened");

    // Create recorder manager
    let recorder_manager = Arc::new(RecorderManager::new(
        session.clone(),
        reductstore_url,
        bucket_name,
    ));

    // Start control interface
    let control_interface =
        ControlInterface::new(session.clone(), recorder_manager.clone(), device_id.clone());

    info!(
        "Starting control interface on recorder/control/{}",
        device_id
    );

    // Run the control interface (blocks until Ctrl+C)
    tokio::select! {
        result = control_interface.run() => {
            if let Err(e) = result {
                tracing::error!("Control interface error: {}", e);
            }
            info!("Control interface stopped");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down");
        }
    }

    // Cleanup
    recorder_manager.shutdown().await?;

    Ok(())
}
