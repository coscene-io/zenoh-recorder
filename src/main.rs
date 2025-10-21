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
use zenoh::prelude::r#async::*;

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
    #[arg(short, long, default_value = "config/default.yaml")]
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
    
    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Zenoh Recorder");
    info!("Loaded configuration from: {:?}", args.config);
    info!("Device ID: {}", recorder_config.recorder.device_id);
    info!("Storage backend: {}", recorder_config.storage.backend);

    // Build Zenoh config
    let mut zenoh_config = Config::default();
    
    // Set mode
    match recorder_config.zenoh.mode.as_str() {
        "peer" => zenoh_config.set_mode(Some(WhatAmI::Peer))?,
        "client" => zenoh_config.set_mode(Some(WhatAmI::Client))?,
        "router" => zenoh_config.set_mode(Some(WhatAmI::Router))?,
        _ => zenoh_config.set_mode(Some(WhatAmI::Peer))?,
    }
    
    // Set connect endpoints
    if let Some(connect_config) = &recorder_config.zenoh.connect {
        let endpoints: Vec<zenoh_config::EndPoint> = connect_config
            .endpoints
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect();
        zenoh_config.connect.endpoints.set(endpoints)?;
    }
    
    // Set listen endpoints
    if let Some(listen_config) = &recorder_config.zenoh.listen {
        let endpoints: Vec<zenoh_config::EndPoint> = listen_config
            .endpoints
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect();
        zenoh_config.listen.endpoints.set(endpoints)?;
    }

    // Open Zenoh session
    let session = Arc::new(
        zenoh::open(zenoh_config)
            .res()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to open Zenoh session: {}", e))?,
    );

    info!("Zenoh session opened");

    // Create storage backend
    let storage_backend = BackendFactory::create(&recorder_config.storage)?;
    info!("Storage backend initialized: {}", storage_backend.backend_type());
    
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
