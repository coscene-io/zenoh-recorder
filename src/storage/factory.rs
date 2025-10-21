// Backend factory for creating storage backends from configuration

use super::backend::StorageBackend;
use super::reductstore::ReductStoreBackend;
use crate::config::{BackendConfig, StorageConfig};
use anyhow::{bail, Result};
use std::sync::Arc;

pub struct BackendFactory;

impl BackendFactory {
    /// Create storage backend from configuration
    pub fn create(config: &StorageConfig) -> Result<Arc<dyn StorageBackend>> {
        match config.backend.as_str() {
            "reductstore" => {
                let backend_config = config
                    .backend_config
                    .as_reductstore()
                    .ok_or_else(|| anyhow::anyhow!("ReductStore config missing"))?;
                
                let backend = ReductStoreBackend::new(backend_config.clone())?;
                Ok(Arc::new(backend))
            }
            
            "filesystem" => {
                // TODO: Implement filesystem backend in Phase 3
                bail!("Filesystem backend not yet implemented. Coming in Phase 3!")
            }
            
            "influxdb" => {
                // TODO: Implement InfluxDB backend (optional)
                bail!("InfluxDB backend not yet implemented. Coming in Phase 3!")
            }
            
            "s3" => {
                // TODO: Implement S3 backend (optional)
                bail!("S3 backend not yet implemented. Coming in Phase 3!")
            }
            
            unknown => bail!(
                "Unknown storage backend: '{}'. Supported: reductstore (filesystem, influxdb, s3 coming soon)",
                unknown
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ReductStoreConfig;
    
    #[test]
    fn test_create_reductstore_backend() {
        let storage_config = StorageConfig {
            backend: "reductstore".to_string(),
            backend_config: BackendConfig::ReductStore {
                reductstore: ReductStoreConfig::default(),
            },
        };
        
        let backend = BackendFactory::create(&storage_config);
        assert!(backend.is_ok());
        assert_eq!(backend.unwrap().backend_type(), "reductstore");
    }
    
    #[test]
    fn test_create_unsupported_backend() {
        let storage_config = StorageConfig {
            backend: "filesystem".to_string(),
            backend_config: BackendConfig::Filesystem {
                filesystem: crate::config::FilesystemConfig::default(),
            },
        };
        
        let backend = BackendFactory::create(&storage_config);
        assert!(backend.is_err());
        assert!(backend.unwrap_err().to_string().contains("not yet implemented"));
    }
    
    #[test]
    fn test_create_unknown_backend() {
        let storage_config = StorageConfig {
            backend: "unknown_backend".to_string(),
            backend_config: BackendConfig::ReductStore {
                reductstore: ReductStoreConfig::default(),
            },
        };
        
        let backend = BackendFactory::create(&storage_config);
        assert!(backend.is_err());
        assert!(backend.unwrap_err().to_string().contains("Unknown storage backend"));
    }
}

