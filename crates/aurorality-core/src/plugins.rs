//! Built-in Rust plugins.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use crate::bridge::NativePlugin;

fn entropy_u32() -> u32 {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    (n ^ n.rotate_left(32) ^ n.rotate_right(17)) as u32
}

// ---------------------------------------------------------------------------
// CorePlugin
// ---------------------------------------------------------------------------

pub struct CorePlugin;

impl NativePlugin for CorePlugin {
    fn id(&self) -> String {
        "core".to_string()
    }

    fn methods(&self) -> Vec<String> {
        vec![
            "echo".into(),
            "ping".into(),
            "timestamp".into(),
            "randomU32".into(),
        ]
    }

    fn invoke(&self, method: &str, payload: &Value) -> Result<Value, String> {
        match method {
            "echo" => Ok(payload.clone()),
            "ping" => Ok(json!({ "pong": true })),
            "timestamp" => {
                let dur = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default();
                Ok(json!({
                    "unixMs": dur.as_millis(),
                    "unixNs": dur.as_nanos().to_string(),
                }))
            }
            "randomU32" => {
                let max = payload
                    .get("max")
                    .and_then(|v| v.as_u64())
                    .filter(|m| *m > 0)
                    .unwrap_or(u32::MAX as u64)
                    .min(u32::MAX as u64);
                let r = entropy_u32() as u64 % max;
                Ok(json!({ "max": max, "value": r }))
            }
            _ => Err("method routed but not handled".to_string()),
        }
    }
}

// ---------------------------------------------------------------------------
// AppPlugin
// ---------------------------------------------------------------------------

pub struct AppPlugin;

impl NativePlugin for AppPlugin {
    fn id(&self) -> String {
        "app".to_string()
    }

    fn methods(&self) -> Vec<String> {
        vec!["version".into(), "platform".into()]
    }

    fn invoke(&self, method: &str, _payload: &Value) -> Result<Value, String> {
        match method {
            "version" => Ok(json!({
                "aurorality": env!("CARGO_PKG_VERSION"),
                "plugin": "app",
            })),
            "platform" => Ok(json!({
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
            })),
            _ => Err("method routed but not handled".to_string()),
        }
    }
}

// ---------------------------------------------------------------------------
// StatsPlugin — text analysis in Rust
// ---------------------------------------------------------------------------

/// Analyzes a text string. Demonstrates a Rust plugin doing real computation.
///
/// Methods:
/// - `analyze` — `{ "text": "..." }` → `{ wordCount, charCount, lineCount, topWord, topWordCount }`
/// - `tokenize` — `{ "text": "..." }` → `{ "tokens": [...] }`
pub struct StatsPlugin;

impl NativePlugin for StatsPlugin {
    fn id(&self) -> String {
        "stats".to_string()
    }

    fn methods(&self) -> Vec<String> {
        vec!["analyze".into(), "tokenize".into()]
    }

    fn invoke(&self, method: &str, payload: &Value) -> Result<Value, String> {
        let text = payload.get("text").and_then(|v| v.as_str()).unwrap_or("");

        match method {
            "analyze" => {
                let char_count = text.chars().count();
                let line_count = text
                    .lines()
                    .count()
                    .max(if text.is_empty() { 0 } else { 1 });
                let words: Vec<&str> = text.split_whitespace().filter(|w| !w.is_empty()).collect();
                let word_count = words.len();

                // frequency count
                let mut freq: HashMap<String, usize> = HashMap::new();
                for w in &words {
                    let key = w
                        .trim_matches(|c: char| !c.is_alphanumeric())
                        .to_lowercase();
                    if !key.is_empty() {
                        *freq.entry(key).or_insert(0) += 1;
                    }
                }
                let (top_word, top_count) = freq
                    .iter()
                    .max_by_key(|(_, &c)| c)
                    .map(|(w, &c)| (w.as_str().to_string(), c))
                    .unwrap_or_default();

                Ok(json!({
                    "wordCount":     word_count,
                    "charCount":     char_count,
                    "lineCount":     line_count,
                    "topWord":       top_word,
                    "topWordCount":  top_count,
                }))
            }
            "tokenize" => {
                let tokens: Vec<&str> = text.split_whitespace().collect();
                Ok(json!({ "tokens": tokens }))
            }
            _ => Err("method routed but not handled".to_string()),
        }
    }
}
