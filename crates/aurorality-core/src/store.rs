//! Simple JSON-on-disk key/value store under `~/Library/Application Support/<bundle_id>/aurorality-store.json`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::AurorError;

static STORE_LOCK: Mutex<()> = Mutex::new(());

fn application_support_file(bundle_id: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home)
        .join("Library/Application Support")
        .join(bundle_id)
        .join("aurorality-store.json")
}

fn load_map(path: &Path) -> Result<HashMap<String, serde_json::Value>, AurorError> {
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let raw = fs::read_to_string(path).map_err(|e| AurorError::PluginError {
        message: format!("read store {path:?}: {e}"),
    })?;
    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| AurorError::PluginError {
        message: format!("parse store JSON: {e}"),
    })?;
    let obj = v.as_object().ok_or_else(|| AurorError::PluginError {
        message: "store root must be a JSON object".into(),
    })?;
    Ok(obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
}

fn save_map(path: &Path, map: &HashMap<String, serde_json::Value>) -> Result<(), AurorError> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| AurorError::PluginError {
            message: format!("create dir {dir:?}: {e}"),
        })?;
    }
    let val = serde_json::Value::Object(map.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
    let raw = serde_json::to_string_pretty(&val).map_err(|e| AurorError::PluginError {
        message: format!("serialize store: {e}"),
    })?;
    fs::write(path, raw).map_err(|e| AurorError::PluginError {
        message: format!("write store {path:?}: {e}"),
    })
}

/// Absolute path of the JSON backing file for `bundle_id`.
pub fn store_path(bundle_id: String) -> String {
    application_support_file(bundle_id.trim())
        .display()
        .to_string()
}

/// Read one JSON value by key; returns `None` when missing.
pub fn store_get(bundle_id: String, key: String) -> Result<Option<String>, AurorError> {
    let _g = STORE_LOCK.lock().unwrap();
    let path = application_support_file(bundle_id.trim());
    let map = load_map(&path)?;
    Ok(map.get(&key).map(|v| v.to_string()))
}

/// Upsert a JSON value (must parse as [`serde_json::Value`]).
pub fn store_set(bundle_id: String, key: String, json: String) -> Result<(), AurorError> {
    let _g = STORE_LOCK.lock().unwrap();
    let path = application_support_file(bundle_id.trim());
    let mut map = load_map(&path)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&json).map_err(|e| AurorError::PluginError {
            message: format!("store_set JSON: {e}"),
        })?;
    map.insert(key, parsed);
    save_map(&path, &map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn get_temp_bundle_id() -> (tempfile::TempDir, String) {
        let dir = tempdir().unwrap();
        // Return absolute path so `PathBuf::join` replaces the prefix completely
        let path = dir.path().to_string_lossy().into_owned();
        (dir, path)
    }

    #[test]
    fn test_store_roundtrip() {
        let (_dir, bundle_id) = get_temp_bundle_id();
        let key = "test_key".to_string();

        // Initially None
        assert_eq!(store_get(bundle_id.clone(), key.clone()).unwrap(), None);

        // Set value
        let json = r#"{"hello": "world"}"#.to_string();
        store_set(bundle_id.clone(), key.clone(), json).unwrap();

        // Get value
        let result = store_get(bundle_id.clone(), key).unwrap().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["hello"], "world");
    }

    #[test]
    fn test_store_nonexistent_key() {
        let (_dir, bundle_id) = get_temp_bundle_id();
        assert_eq!(
            store_get(bundle_id, "missing_key".to_string()).unwrap(),
            None
        );
    }

    #[test]
    fn test_store_invalid_json() {
        let (_dir, bundle_id) = get_temp_bundle_id();
        let err = store_set(bundle_id, "key".to_string(), "invalid".to_string()).unwrap_err();
        assert!(matches!(err, AurorError::PluginError { .. }));
    }

    #[test]
    fn test_store_update_existing_key() {
        let (_dir, bundle_id) = get_temp_bundle_id();
        let key = "update_key".to_string();

        store_set(bundle_id.clone(), key.clone(), r#"{"val": 1}"#.to_string()).unwrap();

        let result1 = store_get(bundle_id.clone(), key.clone()).unwrap().unwrap();
        let parsed1: serde_json::Value = serde_json::from_str(&result1).unwrap();
        assert_eq!(parsed1["val"], 1);

        store_set(bundle_id.clone(), key.clone(), r#"{"val": 2}"#.to_string()).unwrap();

        let result2 = store_get(bundle_id, key).unwrap().unwrap();
        let parsed2: serde_json::Value = serde_json::from_str(&result2).unwrap();
        assert_eq!(parsed2["val"], 2);
    }
}
