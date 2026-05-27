//! `JsPlugin` — a `NativePlugin` backed by a JavaScriptCore runtime.
//!
//! Each `JsPlugin` owns one JSC global context (per plugin, not per method call)
//! so top-level state defined by the JS code persists across invocations.

use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::bridge::NativePlugin;

use super::runtime::JscRuntime;

/// aurorality-lite.js — embedded at compile time, auto-injected before every JS plugin.
const AURORALITY_LITE: &str = include_str!("aurorality-lite.js");

pub struct JsPlugin {
    plugin_id: String,
    exported_methods: Vec<String>,
    runtime: Arc<Mutex<JscRuntime>>,
}

impl JsPlugin {
    /// Create a plugin from a JS source string.
    ///
    /// `aurorality-lite.js` is automatically prepended so `$` state helpers
    /// are always available — like jQuery for `.crepus` templates.
    ///
    /// - `id`: plugin identifier (used in `plugin_invoke` calls)
    /// - `code`: JS source; all top-level `function` declarations become callable methods
    ///
    /// Returns `Err` if the JS fails to parse/execute during the initial load.
    pub fn from_code(id: &str, code: &str) -> Result<Self, String> {
        let full_code = format!("{}\n{}", AURORALITY_LITE, code);
        let methods = extract_fn_names(&full_code);
        let mut rt = JscRuntime::new();
        rt.install_bridge_callback();
        rt.load_code(&full_code)?;
        Ok(Self {
            plugin_id: id.to_string(),
            exported_methods: methods,
            runtime: Arc::new(Mutex::new(rt)),
        })
    }
}

impl NativePlugin for JsPlugin {
    fn id(&self) -> String {
        self.plugin_id.clone()
    }

    fn methods(&self) -> Vec<String> {
        self.exported_methods.clone()
    }

    fn invoke(&self, method: &str, payload: &Value) -> Result<Value, String> {
        let payload_str = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
        let result_str = self
            .runtime
            .lock()
            .map_err(|e| format!("runtime mutex poisoned: {e}"))?
            .call_fn(method, &payload_str)?;
        serde_json::from_str(&result_str)
            .map_err(|e| format!("JS result is not valid JSON: {e} (got: {result_str:?})"))
    }
}

/// Extract top-level function names from JS source.
///
/// Matches lines starting with `function name(` or `export function name(`.
/// Arrow functions and methods are intentionally excluded — only plain
/// `function` declarations define named plugin methods.
pub(crate) fn extract_fn_names(code: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in code.lines() {
        let trimmed = line.trim();
        let rest = if let Some(r) = trimmed.strip_prefix("export function ") {
            r
        } else if let Some(r) = trimmed.strip_prefix("function ") {
            r
        } else {
            continue;
        };
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            names.push(name);
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_names() {
        let code = "export function increment(p) { return p; }\nfunction decrement(p) { return p; }\nconst x = () => {};";
        let names = extract_fn_names(code);
        assert_eq!(names, vec!["increment", "decrement"]);
    }
}
