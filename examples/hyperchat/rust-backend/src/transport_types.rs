//! Shared transport types for HyperChat FFI.

/// Health snapshot returned by any transport.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransportHealth {
    pub id: String,
    pub name: String,
    pub role: String,
    pub connected: bool,
    pub latency_ms: u64,
    pub last_error: Option<String>,
}

/// Transport metadata for discovery / dashboard display.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransportInfo {
    pub id: String,
    pub name: String,
    pub role: String,
    pub trust: String,
    pub latency: u64,
}

/// Message envelope exchanged between transports.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransportMessage {
    pub id: String,
    pub text: String,
    pub transport: String,
    pub status: String,
    pub metadata: serde_json::Value,
}

pub fn envelope_ok(data: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "ok": true, "data": data })
}

pub fn envelope_err(msg: &str) -> serde_json::Value {
    serde_json::json!({ "ok": false, "error": msg })
}
