//! Facade tests: config parsing and config-keyed disk construction.

use std::path::PathBuf;

use rustasea_storage::{StorageError, StorageFacadeConfig, StorageManager};

/// Unique scratch directory for a test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rustasea-facade-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    dir
}

#[test]
fn parses_workspace_storage_config() {
    let config = StorageFacadeConfig::from_toml(include_str!("../../../config/storage.toml"))
        .expect("config/storage.toml must parse");
    assert_eq!(config.default.as_deref(), Some("local"));
    assert!(config.disks.contains_key("local"));
    assert!(config.disks.contains_key("archive"));
    assert!(config.read_through.is_none());
}

#[test]
fn builds_local_disks_and_round_trips() {
    let root = scratch("roundtrip");
    let archive = scratch("roundtrip-archive");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&archive).unwrap();
    let toml = format!(
        r#"
[storage]
default = "local"

[storage.disks.local]
driver = "local"
root = "{root}"

[storage.disks.archive]
driver = "local"
root = "{archive}"
"#,
        root = root.display(),
        archive = archive.display(),
    );

    let manager = StorageManager::from_toml(&toml).unwrap();
    assert!(manager.disk("archive").is_ok());

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        manager.put("a/b.txt", b"hello").await.unwrap();
        assert!(manager.exists("a/b.txt").await.unwrap());
        assert_eq!(manager.get("a/b.txt").await.unwrap(), b"hello".to_vec());
        manager.delete("a/b.txt").await.unwrap();
        assert!(!manager.exists("a/b.txt").await.unwrap());
    });
}

#[test]
fn from_toml_file_reads_document() {
    let root = scratch("file");
    std::fs::create_dir_all(&root).unwrap();
    let path = scratch("file-manifest").with_extension("toml");
    std::fs::write(
        &path,
        format!(
            "[storage]\ndefault = \"local\"\n\n[storage.disks.local]\ndriver = \"local\"\nroot = \"{}\"\n",
            root.display()
        ),
    )
    .unwrap();

    let manager = StorageManager::from_toml_file(&path).unwrap();
    assert!(manager.disk("local").is_ok());
    std::fs::remove_file(&path).ok();
}

#[test]
fn unknown_driver_is_a_config_error() {
    let toml = r#"
[storage]
default = "local"

[storage.disks.local]
driver = "ftp"
host = "example.com"
"#;
    let err = StorageManager::from_toml(toml)
        .err()
        .expect("unknown driver must fail");
    assert!(matches!(err, StorageError::Config(_)), "got {err:?}");
}

#[test]
fn missing_default_and_read_through_is_rejected() {
    let toml = r#"
[storage.disks.local]
driver = "local"
root = "/tmp/rustasea-facade-none"
"#;
    let err = StorageManager::from_toml(toml)
        .err()
        .expect("unknown driver must fail");
    assert!(matches!(err, StorageError::Config(_)), "got {err:?}");
}

#[test]
fn read_through_primary_must_exist() {
    let root = scratch("missing-primary");
    std::fs::create_dir_all(&root).unwrap();
    let toml = format!(
        r#"
[storage]
default = "local"

[storage.read_through]
primary = "s3"
fallback = "local"
copy_back = false

[storage.disks.local]
driver = "local"
root = "{root}"
"#,
        root = root.display(),
    );
    let err = StorageManager::from_toml(&toml)
        .err()
        .expect("config must be rejected");
    assert!(matches!(err, StorageError::Config(_)), "got {err:?}");
}

#[cfg(not(any(feature = "aws", feature = "gcp", feature = "azure")))]
#[test]
fn cloud_drivers_require_their_feature() {
    for (driver, feature) in [("s3", "aws"), ("gcs", "gcp"), ("azure", "azure")] {
        let toml = format!(
            r#"
[storage]
default = "local"

[storage.disks.local]
driver = "local"
root = "/tmp/rustasea-facade-cloud"

[storage.disks.cloud]
driver = "{driver}"
bucket = "example"
account = "example"
container = "example"
"#
        );
        let err = StorageManager::from_toml(&toml)
            .err()
            .expect("config must be rejected");
        assert!(
            matches!(err, StorageError::StoreUnavailable(_)),
            "driver {driver}: got {err:?}"
        );
        assert!(
            err.to_string().contains(feature),
            "driver {driver}: error must name the `{feature}` feature: {err}"
        );
    }
}

#[cfg(all(feature = "aws", feature = "gcp", feature = "azure"))]
#[test]
fn cloud_disk_definitions_parse_with_features() {
    let toml = r#"
[storage]
default = "local"

[storage.disks.local]
driver = "local"
root = "/tmp/rustasea-facade-cloud"

[storage.disks.s3]
driver = "s3"
bucket = "example"
region = "us-east-1"

[storage.disks.gcs]
driver = "gcs"
bucket = "example"

[storage.disks.azure]
driver = "azure"
account = "example"
container = "example"
"#;
    let config = StorageFacadeConfig::from_toml(toml).unwrap();
    assert_eq!(config.disks.len(), 4);
}
