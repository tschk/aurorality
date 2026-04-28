//! `aurorality-core` — UniFFI-exported API for the SwiftUI + Rust crepuscularity shell.
//!
//! Swift imports this library and calls:
//! - [`render_template`] — parse a `.crepus` template and return the `ViewIr` as JSON
//! - [`plugin_invoke`]   — call a built-in Rust plugin
//!
//! Additional plugins written in Rust can be registered by calling
//! [`bridge::Bridge::new`] directly (not yet exposed to Swift — use Swift plugins
//! via the `AurorPlugin` protocol for custom logic).

pub mod bridge;
pub mod plugins;
pub mod render;

uniffi::setup_scaffolding!();

// ---------------------------------------------------------------------------
// Error type (must be exposed so UniFFI can generate a Swift enum for it)
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum AurorError {
    #[error("render error: {message}")]
    RenderError { message: String },
    #[error("invalid context: {message}")]
    InvalidContext { message: String },
    #[error("plugin error: {message}")]
    PluginError { message: String },
}

// ---------------------------------------------------------------------------
// UniFFI exports
// ---------------------------------------------------------------------------

/// Render a `.crepus` template string to a compact `ViewIr` JSON string.
///
/// `context_json` must be a JSON object (`"{}"` for no variables).
/// Returns the serialised [`crepuscularity_native::ViewIr`] on success.
#[uniffi::export]
pub fn render_template(template: String, context_json: String) -> Result<String, AurorError> {
    render::render(&template, &context_json)
}

/// Invoke a built-in Rust plugin.
///
/// Returns a JSON envelope: `{ "ok": true, "data": ... }` or `{ "ok": false, "error": "..." }`.
#[uniffi::export]
pub fn plugin_invoke(
    plugin_id: String,
    method: String,
    payload_json: String,
) -> Result<String, AurorError> {
    bridge::invoke(&plugin_id, &method, &payload_json)
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
        // span with one text child → ViewNode::Text with interpolated content.
        let template = "span\n  \"Hello, {name}!\"";
        let ctx = r#"{"name": "Aurorality"}"#;
        let result = render_template(template.to_string(), ctx.to_string()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["root"][0]["kind"], "text");
        assert!(v["root"][0]["content"]
            .as_str()
            .unwrap()
            .contains("Aurorality"));
    }

    #[test]
    fn core_ping() {
        let result =
            plugin_invoke("core".into(), "ping".into(), "{}".into()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["ok"], true);
        assert_eq!(v["data"]["pong"], true);
    }

    #[test]
    fn unknown_plugin_error() {
        let result =
            plugin_invoke("nope".into(), "ping".into(), "{}".into()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["ok"], false);
    }
}
