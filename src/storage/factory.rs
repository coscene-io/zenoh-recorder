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

// Backend factory for creating storage backends from configuration

use super::backend::StorageBackend;
use super::filesystem::FilesystemBackend;
use super::reductstore::ReductStoreBackend;
use crate::config::StorageConfig;
use anyhow::{bail, Result};
use std::sync::Arc;

#[cfg(test)]
use crate::config::BackendConfig;

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
                let backend_config = config
                    .backend_config
                    .as_filesystem()
                    .ok_or_else(|| anyhow::anyhow!("Filesystem config missing"))?;

                let backend = FilesystemBackend::new(backend_config.clone())?;
                Ok(Arc::new(backend))
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
                "Unknown storage backend: '{}'. Supported: reductstore, filesystem (influxdb, s3 coming soon)",
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
    fn test_create_filesystem_backend() {
        let storage_config = StorageConfig {
            backend: "filesystem".to_string(),
            backend_config: BackendConfig::Filesystem {
                filesystem: crate::config::FilesystemConfig::default(),
            },
        };

        let backend = BackendFactory::create(&storage_config);
        assert!(backend.is_ok());
        assert_eq!(backend.unwrap().backend_type(), "filesystem");
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
        if let Err(e) = backend {
            assert!(e.to_string().contains("Unknown storage backend"));
        }
    }
}
