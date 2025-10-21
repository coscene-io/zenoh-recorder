// Storage backend module
//
// Provides a trait-based abstraction for storage backends,
// allowing the recorder to write to different storage systems
// (ReductStore, filesystem, InfluxDB, S3, etc.)
//
// This module focuses on WRITE-ONLY operations.
// Users should query backends directly using their specialized tools.

pub mod backend;
pub mod factory;
pub mod reductstore;

pub use backend::StorageBackend;
pub use factory::BackendFactory;
pub use reductstore::{topic_to_entry_name, ReductStoreBackend};

// Re-export for backward compatibility
pub use reductstore::ReductStoreBackend as ReductStoreClient;

