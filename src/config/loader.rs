// Configuration loader with environment variable substitution

use super::types::*;
use anyhow::{bail, Context, Result};
use regex::Regex;
use std::path::Path;

pub struct ConfigLoader;

impl ConfigLoader {
    /// Load configuration from file with environment variable substitution
    pub fn load<P: AsRef<Path>>(path: P) -> Result<RecorderConfig> {
        let content = std::fs::read_to_string(path.as_ref())
            .context("Failed to read config file")?;
        
        // Substitute environment variables
        let content = Self::substitute_env_vars(&content);
        
        // Parse YAML
        let config: RecorderConfig = serde_yaml::from_str(&content)
            .context("Failed to parse YAML configuration")?;
        
        // Validate configuration
        Self::validate(&config)?;
        
        Ok(config)
    }
    
    /// Substitute ${VAR} and ${VAR:-default} patterns with environment variables
    /// 
    /// Examples:
    /// - ${HOME} -> /home/user
    /// - ${DEVICE_ID:-robot-001} -> robot-001 (if DEVICE_ID not set)
    fn substitute_env_vars(content: &str) -> String {
        let re = Regex::new(r"\$\{([^}:]+)(?::-([^}]+))?\}").unwrap();
        
        re.replace_all(content, |caps: &regex::Captures| {
            let var_name = &caps[1];
            let default_value = caps.get(2).map(|m| m.as_str());
            
            match std::env::var(var_name) {
                Ok(value) => value,
                Err(_) => {
                    if let Some(default) = default_value {
                        default.to_string()
                    } else {
                        // Keep original if no default and var not found
                        format!("${{{}}}", var_name)
                    }
                }
            }
        }).to_string()
    }
    
    /// Validate configuration
    fn validate(config: &RecorderConfig) -> Result<()> {
        // Validate flush policy
        if config.recorder.flush_policy.max_buffer_size_bytes == 0 {
            bail!("flush_policy.max_buffer_size_bytes must be > 0");
        }
        
        if config.recorder.flush_policy.max_buffer_duration_seconds == 0 {
            bail!("flush_policy.max_buffer_duration_seconds must be > 0");
        }
        
        // Validate compression level
        if config.recorder.compression.default_level > 4 {
            bail!("compression.default_level must be 0-4");
        }
        
        // Validate backend
        match config.storage.backend.as_str() {
            "reductstore" => {
                if config.storage.backend_config.as_reductstore().is_none() {
                    bail!("reductstore backend selected but reductstore config missing");
                }
            }
            "filesystem" => {
                if config.storage.backend_config.as_filesystem().is_none() {
                    bail!("filesystem backend selected but filesystem config missing");
                }
            }
            unknown => bail!("Unknown backend: '{}'. Supported: reductstore, filesystem", unknown),
        }
        
        // Validate worker count
        if config.recorder.workers.flush_workers == 0 {
            bail!("workers.flush_workers must be > 0");
        }
        
        if config.recorder.workers.queue_capacity == 0 {
            bail!("workers.queue_capacity must be > 0");
        }
        
        // Validate device_id is not empty
        if config.recorder.device_id.is_empty() {
            bail!("recorder.device_id cannot be empty");
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_env_var_substitution() {
        // Set test environment variable
        std::env::set_var("TEST_VAR", "test_value");
        
        let input = "url: ${TEST_VAR}";
        let output = ConfigLoader::substitute_env_vars(input);
        assert_eq!(output, "url: test_value");
        
        std::env::remove_var("TEST_VAR");
    }
    
    #[test]
    fn test_env_var_with_default() {
        // Don't set TEST_VAR2
        std::env::remove_var("TEST_VAR2");
        
        let input = "device_id: ${TEST_VAR2:-default-device}";
        let output = ConfigLoader::substitute_env_vars(input);
        assert_eq!(output, "device_id: default-device");
    }
    
    #[test]
    fn test_validation_invalid_buffer_size() {
        let mut config = RecorderConfig::default();
        config.recorder.flush_policy.max_buffer_size_bytes = 0;
        
        let result = ConfigLoader::validate(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("max_buffer_size_bytes"));
    }
    
    #[test]
    fn test_validation_invalid_compression_level() {
        let mut config = RecorderConfig::default();
        config.recorder.compression.default_level = 10;
        
        let result = ConfigLoader::validate(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("compression"));
    }
}

