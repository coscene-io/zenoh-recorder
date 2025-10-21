// Configuration module for zenoh-recorder
//
// Provides:
// - YAML configuration file loading
// - Environment variable substitution
// - Configuration validation
// - Default values

pub mod types;
mod loader;

pub use types::*;
pub use loader::ConfigLoader;

use anyhow::{Context, Result};
use std::path::Path;

/// Load configuration from a YAML file
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<RecorderConfig> {
    ConfigLoader::load(path).context("Failed to load configuration")
}

/// Load configuration with environment variable overrides
pub fn load_config_with_env<P: AsRef<Path>>(path: P) -> Result<RecorderConfig> {
    let mut config = load_config(path)?;
    
    // Allow environment variables to override config values
    if let Ok(device_id) = std::env::var("DEVICE_ID") {
        config.recorder.device_id = device_id;
    }
    
    if let Ok(reduct_url) = std::env::var("REDUCTSTORE_URL") {
        if let Some(reduct_config) = config.storage.backend_config.as_reductstore_mut() {
            reduct_config.url = reduct_url;
        }
    }
    
    if let Ok(api_token) = std::env::var("REDUCT_API_TOKEN") {
        if let Some(reduct_config) = config.storage.backend_config.as_reductstore_mut() {
            reduct_config.api_token = Some(api_token);
        }
    }
    
    Ok(config)
}

