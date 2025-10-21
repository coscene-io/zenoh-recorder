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
pub mod filesystem;
pub mod reductstore;

pub use backend::StorageBackend;
pub use factory::BackendFactory;
pub use filesystem::FilesystemBackend;
pub use reductstore::{topic_to_entry_name, ReductStoreBackend};

// Re-export for backward compatibility
pub use reductstore::ReductStoreBackend as ReductStoreClient;

