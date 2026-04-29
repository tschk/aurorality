//! JavaScriptCore plugin backend — optional, enabled via `--features js`.
//!
//! Provides `load_plugin(id, code)` which creates a `JsPlugin` backed by a
//! JSC runtime and registers it with the global bridge.

pub mod plugin;
pub mod runtime;

use std::sync::Arc;

/// Load a JavaScript source string as a named plugin and register it with the global bridge.
///
/// Top-level `function` declarations in `code` become callable plugin methods.
/// JS code has access to `globalThis.aurorality.invoke(pluginId, method, payloadJson)`.
pub fn load_plugin(id: &str, code: &str) -> Result<(), String> {
    let js_plugin = plugin::JsPlugin::from_code(id, code)?;
    crate::bridge::register_plugin(Arc::new(js_plugin));
    Ok(())
}
