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
