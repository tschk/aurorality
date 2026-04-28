//! Plugin registry — routes `plugin_invoke` calls to registered `NativePlugin` impls.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{json, Value};

use crate::plugins::{AppPlugin, CorePlugin, StatsPlugin};
use crate::AurorError;

pub trait NativePlugin: Send + Sync {
    fn id(&self) -> &'static str;
    fn methods(&self) -> &'static [&'static str];
    fn invoke(&self, method: &str, payload: &Value) -> Result<Value, String>;
}

pub struct Bridge {
    plugins: HashMap<String, Arc<dyn NativePlugin>>,
}

impl Bridge {
    pub fn new() -> Self {
        let mut plugins: HashMap<String, Arc<dyn NativePlugin>> = HashMap::new();
        let core: Arc<dyn NativePlugin> = Arc::new(CorePlugin);
        plugins.insert(core.id().to_string(), core);
        let app: Arc<dyn NativePlugin> = Arc::new(AppPlugin);
        plugins.insert(app.id().to_string(), app);
        let stats: Arc<dyn NativePlugin> = Arc::new(StatsPlugin);
        plugins.insert(stats.id().to_string(), stats);
        Self { plugins }
    }

    /// Dispatch a call and return a JSON envelope:
    /// `{ "ok": true, "data": ... }` or `{ "ok": false, "error": "..." }`.
    pub fn invoke_envelope(&self, plugin_id: &str, method: &str, payload: &Value) -> Value {
        match self.invoke_inner(plugin_id, method, payload) {
            Ok(data) => json!({ "ok": true, "data": data }),
            Err(e) => json!({ "ok": false, "error": e }),
        }
    }

    fn invoke_inner(
        &self,
        plugin_id: &str,
        method: &str,
        payload: &Value,
    ) -> Result<Value, String> {
        let plugin = self
            .plugins
            .get(plugin_id)
            .ok_or_else(|| format!("no plugin registered as {plugin_id:?}"))?;

        if !plugin.methods().contains(&method) {
            return Err(format!("plugin {plugin_id:?} has no method {method:?}"));
        }

        plugin.invoke(method, payload)
    }
}

impl Default for Bridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Global bridge instance used by UniFFI exports.
pub fn global_bridge() -> &'static Bridge {
    static BRIDGE: std::sync::OnceLock<Bridge> = std::sync::OnceLock::new();
    BRIDGE.get_or_init(Bridge::new)
}

/// Invoke a plugin via the global bridge and return the JSON envelope string.
pub fn invoke(plugin_id: &str, method: &str, payload_json: &str) -> Result<String, AurorError> {
    let payload: Value = serde_json::from_str(payload_json).map_err(|e| AurorError::InvalidContext {
        message: format!("invalid payload JSON: {e}"),
    })?;
    let result = global_bridge().invoke_envelope(plugin_id, method, &payload);
    serde_json::to_string(&result).map_err(|e| AurorError::RenderError {
        message: e.to_string(),
    })
}
