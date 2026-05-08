//! `aurorality-core` — eqswift-exported API for the SwiftUI + Rust crepuscularity shell.
//!
//! Swift imports this library and calls:
//! - [`render_template`] — parse a `.crepus` template and return the `ViewIr` as JSON
//! - [`plugin_invoke`]   — call a built-in Rust plugin
//!
//! Additional plugins written in Rust can be registered by calling
//! [`bridge::Bridge::new`] directly (not yet exposed to Swift — use Swift plugins
//! via the `AurorPlugin` protocol for custom logic).

pub mod bridge;
pub mod color;
#[cfg(feature = "js")]
pub mod js;
pub mod mutations;
pub mod plugins;
pub mod render;
pub mod store;
pub mod text;
pub mod transport;

pub use color::ResolvedColor;

eqswift::setup!();

// ---------------------------------------------------------------------------
// Error type (must be exposed so eqswift can generate a Swift enum for it)
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error, eqswift::Error)]
pub enum AurorError {
    #[error("render error: {message}")]
    RenderError { message: String },
    #[error("invalid context: {message}")]
    InvalidContext { message: String },
    #[error("plugin error: {message}")]
    PluginError { message: String },
}

// ---------------------------------------------------------------------------
// eqswift exports
// ---------------------------------------------------------------------------

/// Render a `.crepus` template string to a compact `ViewIr` JSON string.
///
/// `context_json` must be a JSON object (`"{}"` for no variables).
/// Returns the serialised [`crepuscularity_native::ViewIr`] on success.
#[eqswift::export]
pub fn render_template(template: String, context_json: String) -> Result<String, AurorError> {
    render::render(&template, &context_json)
}

/// Invoke a built-in Rust plugin.
///
/// Returns a JSON envelope: `{ "ok": true, "data": ... }` or `{ "ok": false, "error": "..." }`.
#[eqswift::export]
pub fn plugin_invoke(
    plugin_id: String,
    method: String,
    payload_json: String,
) -> Result<String, AurorError> {
    bridge::invoke(&plugin_id, &method, &payload_json)
}

/// Apply a JSON-encoded `[IrMutation]` array to a JSON-encoded `ViewIr`.
/// Returns updated `ViewIr` JSON. Used by Swift's `ViewIr.applying(_:)`.
#[eqswift::export]
pub fn apply_mutations(ir_json: String, mutations_json: String) -> Result<String, AurorError> {
    mutations::apply(&ir_json, &mutations_json)
        .map_err(|message| AurorError::RenderError { message })
}

/// Resolve a CSS color string to RGBA components.
/// Returns `None` for `"primary"`, `"secondary"`, and unknown strings
/// (Swift handles those as `Color.primary` / `Color.secondary`).
#[eqswift::export]
pub fn resolve_color(css: String) -> Option<color::ResolvedColor> {
    color::resolve_color(&css)
}

/// Apply a CSS text-transform to a string.
#[eqswift::export]
pub fn transform_text(content: String, transform: String) -> String {
    text::transform_text(&content, &transform)
}

/// Path to the on-disk JSON store for `bundle_id` (macOS Application Support).
#[eqswift::export]
pub fn auror_store_path(bundle_id: String) -> String {
    store::store_path(bundle_id)
}

/// Read a JSON payload by key from the store file.
#[eqswift::export]
pub fn auror_store_get(bundle_id: String, key: String) -> Result<Option<String>, AurorError> {
    store::store_get(bundle_id, key)
}

/// Write a JSON value for `key` (string must parse as JSON).
#[eqswift::export]
pub fn auror_store_set(bundle_id: String, key: String, json: String) -> Result<(), AurorError> {
    store::store_set(bundle_id, key, json)
}

/// Load or replace a JavaScript plugin (JSC). When `aurorality_core` is built without the
/// `js` feature, this returns an error — the Swift bindings still link.
#[eqswift::export]
pub fn js_load_plugin(id: String, code: String) -> Result<(), AurorError> {
    #[cfg(feature = "js")]
    {
        return js::load_plugin(&id, &code).map_err(|message| AurorError::PluginError { message });
    }
    #[cfg(not(feature = "js"))]
    {
        let _ = (id, code);
        Err(AurorError::PluginError {
            message: "aurorality_core was built without the `js` Cargo feature".into(),
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_simple_text() {
        // A bare quoted string is a Node::Text → ViewNode::Text.
        let template = "\"Hello, world!\"";
        let result = render_template(template.to_string(), "{}".to_string()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["version"], 2);
        assert_eq!(v["root"][0]["kind"], "text");
        assert_eq!(v["root"][0]["content"], "Hello, world!");
    }

    #[test]
    fn render_with_context() {
        // Rendering with context should remain stable even when fields are unused.
        let template = "\"Hello, {name}!\"";
        let ctx = r#"{"name": "Aurorality"}"#;
        let result = render_template(template.to_string(), ctx.to_string()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["version"], 2);
        assert!(v["root"].is_array());
    }

    #[test]
    fn core_ping() {
        let result = plugin_invoke("core".into(), "ping".into(), "{}".into()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["ok"], true);
        assert_eq!(v["data"]["pong"], true);
    }

    #[test]
    fn unknown_plugin_error() {
        let result = plugin_invoke("nope".into(), "ping".into(), "{}".into()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["ok"], false);
    }
}
