//! Config-keyed disk construction for `config/storage.toml`.
//!
//! Parses the `[storage]` table into [`StorageFacadeConfig`] and turns each
//! named disk definition into a live [`ManagedDisk`]. Local disks resolve to a
//! filesystem root; `s3`/`gcs`/`azure` disks are backed by `object_store` and
//! are feature-gated (`aws`, `gcp`, `azure`). Building a cloud disk without its
//! feature surfaces [`StorageError::StoreUnavailable`] naming the feature, so
//! misconfiguration is explicit rather than a silent fallback.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use serde::Deserialize;

use crate::disk::LocalDisk;
use crate::error::{Result, StorageError};
use crate::manager::{ManagedDisk, StorageConfig, StorageManager};

/// Top-level `[storage]` configuration document.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
pub struct StorageFacadeConfig {
    /// Disk served when no read-through pair is configured.
    #[serde(default)]
    pub default: Option<String>,
    /// Named disk definitions keyed by disk name.
    #[serde(default)]
    pub disks: HashMap<String, DiskDefinition>,
    /// Optional read-through routing (`{ primary, fallback, copy_back }`).
    #[serde(default)]
    pub read_through: Option<StorageConfig>,
}

impl StorageFacadeConfig {
    /// Parse a `config/storage.toml` document.
    pub fn from_toml(input: &str) -> Result<Self> {
        let document: StorageDocument =
            toml::from_str(input).map_err(|e| StorageError::Config(e.to_string()))?;
        Ok(document.storage)
    }

    /// Parse a `config/storage.toml` file from disk.
    pub fn from_toml_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path)
            .map_err(|e| StorageError::Config(format!("{}: {e}", path.display())))?;
        Self::from_toml(&contents)
    }
}

/// Internal wrapper matching the document's top-level `[storage]` table.
#[derive(Debug, Deserialize)]
struct StorageDocument {
    /// The single `[storage]` table.
    storage: StorageFacadeConfig,
}

/// A single disk definition, tagged by its `driver` key.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "driver", rename_all = "lowercase")]
pub enum DiskDefinition {
    /// Local filesystem root.
    Local(LocalDiskConfig),
    /// Amazon S3 bucket (requires the `aws` feature).
    S3(S3DiskConfig),
    /// Google Cloud Storage bucket (requires the `gcp` feature).
    Gcs(GcsDiskConfig),
    /// Azure Blob container (requires the `azure` feature).
    Azure(AzureDiskConfig),
}

impl DiskDefinition {
    /// Instantiate this disk under `name`.
    pub fn build(&self, name: &str) -> Result<Arc<dyn ManagedDisk>> {
        match self {
            DiskDefinition::Local(config) => Ok(Arc::new(LocalDisk::new(config.root.clone()))),
            DiskDefinition::S3(config) => build_s3(config, name),
            DiskDefinition::Gcs(config) => build_gcs(config, name),
            DiskDefinition::Azure(config) => build_azure(config, name),
        }
    }
}

/// Configuration for a local filesystem disk.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct LocalDiskConfig {
    /// Root directory that confines every key.
    pub root: String,
}

/// Configuration for an S3 disk.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct S3DiskConfig {
    /// Bucket name.
    pub bucket: String,
    /// AWS region (falls back to the SDK default when absent).
    #[serde(default)]
    pub region: Option<String>,
    /// Explicit access key id (prefer environment/secret manager).
    #[serde(default)]
    pub access_key_id: Option<String>,
    /// Explicit secret access key (prefer environment/secret manager).
    #[serde(default)]
    pub secret_access_key: Option<String>,
    /// Custom endpoint for S3-compatible services (MinIO, R2).
    #[serde(default)]
    pub endpoint: Option<String>,
}

/// Configuration for a Google Cloud Storage disk.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct GcsDiskConfig {
    /// Bucket name.
    pub bucket: String,
    /// Path to a service-account JSON key.
    #[serde(default)]
    pub service_account_path: Option<String>,
}

/// Configuration for an Azure Blob Storage disk.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct AzureDiskConfig {
    /// Storage account name.
    pub account: String,
    /// Blob container name.
    pub container: String,
    /// Explicit access key (prefer environment/secret manager).
    #[serde(default)]
    pub access_key: Option<String>,
}

impl StorageManager {
    /// Build a manager from a facade configuration, instantiating every disk.
    pub fn from_facade(config: &StorageFacadeConfig) -> Result<Self> {
        let mut disks: HashMap<String, Arc<dyn ManagedDisk>> = HashMap::new();
        for (name, definition) in &config.disks {
            disks.insert(name.clone(), definition.build(name)?);
        }
        let routing = match &config.read_through {
            Some(read_through) => read_through.clone(),
            None => {
                let default = config.default.clone().ok_or_else(|| {
                    StorageError::Config("`default` or `read_through` is required".into())
                })?;
                StorageConfig {
                    primary: default.clone(),
                    fallback: default,
                    copy_back: false,
                }
            }
        };
        if !disks.contains_key(&routing.primary) {
            return Err(StorageError::Config(format!(
                "primary disk `{}` is not defined",
                routing.primary
            )));
        }
        if !disks.contains_key(&routing.fallback) {
            return Err(StorageError::Config(format!(
                "fallback disk `{}` is not defined",
                routing.fallback
            )));
        }
        Ok(StorageManager::from_parts(disks, routing))
    }

    /// Build a manager from a `config/storage.toml` document string.
    pub fn from_toml(input: &str) -> Result<Self> {
        Self::from_facade(&StorageFacadeConfig::from_toml(input)?)
    }

    /// Build a manager from a `config/storage.toml` file path.
    pub fn from_toml_file(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_facade(&StorageFacadeConfig::from_toml_file(path)?)
    }
}

/// Build an S3-backed disk.
#[cfg(feature = "aws")]
fn build_s3(config: &S3DiskConfig, name: &str) -> Result<Arc<dyn ManagedDisk>> {
    let mut builder =
        object_store::aws::AmazonS3Builder::new().with_bucket_name(config.bucket.clone());
    if let Some(region) = &config.region {
        builder = builder.with_region(region.clone());
    }
    if let Some(key_id) = &config.access_key_id {
        builder = builder.with_access_key_id(key_id.clone());
    }
    if let Some(secret) = &config.secret_access_key {
        builder = builder.with_secret_access_key(secret.clone());
    }
    if let Some(endpoint) = &config.endpoint {
        builder = builder.with_endpoint(endpoint.clone());
    }
    let store = builder
        .build()
        .map_err(|e| StorageError::StoreUnavailable(format!("disk {name}: {e}")))?;
    Ok(Arc::new(crate::ObjectDisk::new(
        Arc::new(store),
        format!("s3://{}", config.bucket),
    )))
}

/// Reject S3 disks when the `aws` feature is disabled.
#[cfg(not(feature = "aws"))]
fn build_s3(_config: &S3DiskConfig, name: &str) -> Result<Arc<dyn ManagedDisk>> {
    Err(StorageError::StoreUnavailable(format!(
        "disk {name}: the `s3` driver requires the `aws` feature"
    )))
}

/// Build a Google Cloud Storage-backed disk.
#[cfg(feature = "gcp")]
fn build_gcs(config: &GcsDiskConfig, name: &str) -> Result<Arc<dyn ManagedDisk>> {
    let mut builder =
        object_store::gcp::GoogleCloudStorageBuilder::new().with_bucket_name(config.bucket.clone());
    if let Some(path) = &config.service_account_path {
        builder = builder.with_service_account_path(path.clone());
    }
    let store = builder
        .build()
        .map_err(|e| StorageError::StoreUnavailable(format!("disk {name}: {e}")))?;
    Ok(Arc::new(crate::ObjectDisk::new(
        Arc::new(store),
        format!("gs://{}", config.bucket),
    )))
}

/// Reject GCS disks when the `gcp` feature is disabled.
#[cfg(not(feature = "gcp"))]
fn build_gcs(_config: &GcsDiskConfig, name: &str) -> Result<Arc<dyn ManagedDisk>> {
    Err(StorageError::StoreUnavailable(format!(
        "disk {name}: the `gcs` driver requires the `gcp` feature"
    )))
}

/// Build an Azure Blob-backed disk.
#[cfg(feature = "azure")]
fn build_azure(config: &AzureDiskConfig, name: &str) -> Result<Arc<dyn ManagedDisk>> {
    let mut builder = object_store::azure::MicrosoftAzureBuilder::new()
        .with_account(config.account.clone())
        .with_container_name(config.container.clone());
    if let Some(key) = &config.access_key {
        builder = builder.with_access_key(key.clone());
    }
    let store = builder
        .build()
        .map_err(|e| StorageError::StoreUnavailable(format!("disk {name}: {e}")))?;
    Ok(Arc::new(crate::ObjectDisk::new(
        Arc::new(store),
        format!("az://{}/{}", config.account, config.container),
    )))
}

/// Reject Azure disks when the `azure` feature is disabled.
#[cfg(not(feature = "azure"))]
fn build_azure(_config: &AzureDiskConfig, name: &str) -> Result<Arc<dyn ManagedDisk>> {
    Err(StorageError::StoreUnavailable(format!(
        "disk {name}: the `azure` driver requires the `azure` feature"
    )))
}
