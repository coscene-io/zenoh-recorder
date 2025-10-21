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

