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

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use zenoh::config::Config;
use zenoh::Wait;

mod buffer;
mod config;
mod control;
mod mcap_writer;
mod protocol;
mod recorder;
mod storage;

use config::load_config_with_env;
use control::ControlInterface;
use recorder::RecorderManager;
use storage::BackendFactory;

/// Zenoh Recorder - Record Zenoh topics to storage backends
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "config/default.toml")]
    config: PathBuf,

    /// Device ID (overrides config file)
    #[arg(short, long)]
    device_id: Option<String>,
}

// Include protobuf definitions
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/sensor_data.rs"));
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let args = Args::parse();

    // Load configuration from file
    let mut recorder_config = load_config_with_env(&args.config)?;

    // Apply CLI overrides
    if let Some(device_id) = args.device_id {
        recorder_config.recorder.device_id = device_id;
    }

    // Initialize tracing with configured level
    let log_level = match recorder_config.logging.level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder().with_max_level(log_level).finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Zenoh Recorder");
    info!("Loaded configuration from: {:?}", args.config);
    info!("Device ID: {}", recorder_config.recorder.device_id);
    info!("Storage backend: {}", recorder_config.storage.backend);

    // Build Zenoh config using insert_json5 API (Zenoh 1.6)
    let mut zenoh_config = Config::default();

    // Set mode (peer, client, or router)
    zenoh_config
        .insert_json5("mode", &format!("\"{}\"", recorder_config.zenoh.mode))
        .map_err(|e| anyhow::anyhow!("Failed to set Zenoh mode: {}", e))?;

    info!("Zenoh mode: {}", recorder_config.zenoh.mode);

    // Set connect endpoints (for connecting to routers/peers)
    if let Some(connect_config) = &recorder_config.zenoh.connect {
        if !connect_config.endpoints.is_empty() {
            let endpoints_json = connect_config
                .endpoints
                .iter()
                .map(|e| format!("\"{}\"", e))
                .collect::<Vec<_>>()
                .join(", ");
            zenoh_config
                .insert_json5("connect/endpoints", &format!("[{}]", endpoints_json))
                .map_err(|e| anyhow::anyhow!("Failed to set connect endpoints: {}", e))?;
            info!("Connect endpoints: {:?}", connect_config.endpoints);
        }
    }

    // Set listen endpoints (for accepting incoming connections)
    if let Some(listen_config) = &recorder_config.zenoh.listen {
        if !listen_config.endpoints.is_empty() {
            let endpoints_json = listen_config
                .endpoints
                .iter()
                .map(|e| format!("\"{}\"", e))
                .collect::<Vec<_>>()
                .join(", ");
            zenoh_config
                .insert_json5("listen/endpoints", &format!("[{}]", endpoints_json))
                .map_err(|e| anyhow::anyhow!("Failed to set listen endpoints: {}", e))?;
            info!("Listen endpoints: {:?}", listen_config.endpoints);
        }
    }

    // Open Zenoh session
    let session = Arc::new(
        zenoh::open(zenoh_config)
            .wait()
            .map_err(|e| anyhow::anyhow!("Failed to open Zenoh session: {}", e))?,
    );

    info!("Zenoh session opened");

    // Create storage backend
    let storage_backend = BackendFactory::create(&recorder_config.storage)?;
    info!(
        "Storage backend initialized: {}",
        storage_backend.backend_type()
    );

    // Initialize storage backend
    storage_backend.initialize().await?;

    // Create recorder manager
    let recorder_manager = Arc::new(RecorderManager::new(
        session.clone(),
        storage_backend,
        recorder_config.clone(),
    ));

    // Start control interface
    let device_id = recorder_config.recorder.device_id.clone();
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
    info!("Zenoh Recorder shut down successfully");

    Ok(())
}
