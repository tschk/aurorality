//! Matrix transport adapter (Client-Server API).
//!
//! ## Configuration (environment)
//! - `MATRIX_HOMESERVER`, `MATRIX_USER_ID`, `MATRIX_ACCESS_TOKEN`, `MATRIX_ROOM_ID`

use crate::transport_types::{
    envelope_err, envelope_ok, TransportHealth, TransportInfo, TransportMessage,
};

use serde::Deserialize;
use serde_json::Value;

const TRANSPORT_ID: &str = "matrix";

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct LoginResponse {
    access_token: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct SyncResponse {
    rooms: Option<JoinRooms>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct JoinRooms {
    join: Value,
}

use std::sync::{Mutex, OnceLock};

static MATRIX_CONFIG: OnceLock<Mutex<MatrixClient>> = OnceLock::new();

pub fn set_matrix_config(
    homeserver: Option<String>,
    user_id: Option<String>,
    access_token: Option<String>,
    room_id: Option<String>,
) {
    let client = MatrixClient {
        homeserver: homeserver
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        user_id: user_id
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        access_token: access_token
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        room_id: room_id
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
    };

    let mut config = MATRIX_CONFIG
        .get_or_init(|| {
            Mutex::new(MatrixClient {
                homeserver: None,
                user_id: None,
                access_token: None,
                room_id: None,
            })
        })
        .lock()
        .unwrap();

    *config = client;
}

#[derive(Clone)]
pub struct MatrixClient {
    homeserver: Option<String>,
    #[allow(dead_code)]
    user_id: Option<String>,
    access_token: Option<String>,
    room_id: Option<String>,
}

impl MatrixClient {
    pub fn current() -> Self {
        MATRIX_CONFIG
            .get_or_init(|| {
                Mutex::new(MatrixClient {
                    homeserver: None,
                    user_id: None,
                    access_token: None,
                    room_id: None,
                })
            })
            .lock()
            .unwrap()
            .clone()
    }

    #[allow(dead_code)]
    pub fn new(homeserver: &str, user_id: &str, access_token: &str, room_id: &str) -> Self {
        Self {
            homeserver: Some(homeserver.to_string()),
            user_id: Some(user_id.to_string()),
            access_token: Some(access_token.to_string()),
            room_id: Some(room_id.to_string()),
        }
    }

    fn configured(&self) -> bool {
        self.homeserver.is_some() && self.access_token.is_some() && self.room_id.is_some()
    }

    /// Homeserver + access token only (room optional — multi-room APIs).
    pub(crate) fn token_ready(&self) -> bool {
        self.homeserver.is_some() && self.access_token.is_some()
    }

    pub(crate) fn homeserver_trimmed(&self) -> Result<String, String> {
        self.homeserver
            .as_ref()
            .map(|s| s.trim_end_matches('/').to_string())
            .ok_or_else(|| "MATRIX_HOMESERVER not set".into())
    }

    pub(crate) fn user_id_owned(&self) -> Option<String> {
        self.user_id.clone()
    }

    fn server_url(&self, path: &str) -> Option<String> {
        Some(format!(
            "{}/_matrix/client/v3/{path}",
            self.homeserver.as_ref()?.trim_end_matches('/')
        ))
    }

    pub(crate) fn auth_get(&self, url: &str) -> Result<Value, String> {
        ureq::get(url)
            .set(
                "Authorization",
                &format!("Bearer {}", self.access_token.as_deref().unwrap_or("")),
            )
            .call()
            .map_err(|e| format!("Matrix GET {url}: {e}"))?
            .into_json()
            .map_err(|e| format!("Matrix parse error: {e}"))
    }

    pub(crate) fn auth_put(&self, url: &str, body: Value) -> Result<Value, String> {
        ureq::put(url)
            .set(
                "Authorization",
                &format!("Bearer {}", self.access_token.as_deref().unwrap_or("")),
            )
            .set("Content-Type", "application/json")
            .send_json(body)
            .map_err(|e| format!("Matrix PUT {url}: {e}"))?
            .into_json()
            .map_err(|e| format!("Matrix parse error: {e}"))
    }

    pub(crate) fn upload_media_raw(
        &self,
        upload_url: &str,
        content_type: &str,
        bytes: &[u8],
    ) -> Result<String, String> {
        let resp = ureq::post(upload_url)
            .set(
                "Authorization",
                &format!("Bearer {}", self.access_token.as_deref().unwrap_or("")),
            )
            .set("Content-Type", content_type)
            .send(bytes)
            .map_err(|e| format!("Matrix upload {upload_url}: {e}"))?;
        let v: Value = resp.into_json().map_err(|e| format!("upload parse: {e}"))?;
        let uri = v["content_uri"]
            .as_str()
            .ok_or_else(|| "missing content_uri".to_string())?;
        Ok(uri.to_string())
    }

    fn health_check(&self) -> Result<TransportHealth, String> {
        if !self.configured() {
            return Ok(TransportHealth {
                id: TRANSPORT_ID.to_string(),
                name: "Matrix".to_string(),
                role: "federation".to_string(),
                connected: false,
                latency_ms: 0,
                last_error: Some("not configured".to_string()),
            });
        }

        let start = std::time::Instant::now();
        let versions_url = format!(
            "{}/_matrix/client/versions",
            self.homeserver.as_deref().unwrap()
        );
        match ureq::get(&versions_url).call() {
            Ok(resp) => {
                let status = resp.status();
                let latency = start.elapsed().as_millis() as u64;
                Ok(TransportHealth {
                    id: TRANSPORT_ID.to_string(),
                    name: "Matrix".to_string(),
                    role: "federation".to_string(),
                    connected: status == 200,
                    latency_ms: latency,
                    last_error: if status == 200 {
                        None
                    } else {
                        Some(format!("HTTP {status}"))
                    },
                })
            }
            Err(e) => Ok(TransportHealth {
                id: TRANSPORT_ID.to_string(),
                name: "Matrix".to_string(),
                role: "federation".to_string(),
                connected: false,
                latency_ms: 0,
                last_error: Some(e.to_string()),
            }),
        }
    }

    fn sync_messages(&self) -> Result<Vec<TransportMessage>, String> {
        let url = self
            .server_url("sync")
            .ok_or("Matrix homeserver not configured")?;
        let mut sync: Value = self.auth_get(&url)?;

        let mut messages = Vec::new();
        let room_id = self.room_id.as_deref().unwrap_or("");
        if let Some(rooms) = sync.get_mut("rooms") {
            if let Some(join) = rooms.get_mut("join") {
                if let Some(room) = join.get_mut(room_id) {
                    if let Some(timeline) = room.get_mut("timeline") {
                        if let Some(events) = timeline.get_mut("events") {
                            if let Value::Array(timeline_arr) = events.take() {
                                for event in timeline_arr {
                                    if event["type"].as_str() == Some("m.room.message") {
                                        let body = event["content"]["body"]
                                            .as_str()
                                            .unwrap_or("(empty)")
                                            .to_string();
                                        messages.push(TransportMessage {
                                            id: event["event_id"]
                                                .as_str()
                                                .unwrap_or("?")
                                                .to_string(),
                                            text: body,
                                            transport: "matrix".to_string(),
                                            status: "synced".to_string(),
                                            metadata: event,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(messages)
    }

    fn send_message(&self, text: &str) -> Result<Value, String> {
        let room_id = self
            .room_id
            .as_ref()
            .ok_or("Matrix room ID not configured")?;
        let txn_id = format!("m{}", timestamp_ms());

        let url = self
            .server_url(&format!("rooms/{room_id}/send/m.room.message/{txn_id}"))
            .ok_or("Matrix homeserver not configured")?;

        let body = ureq::json!({
            "msgtype": "m.text",
            "body": text,
        });

        let resp = self.auth_put(&url, body)?;
        Ok(resp)
    }

    /// Same routing as Aurorality’s Matrix plugin (`NativePlugin::invoke`), without the bridge.
    pub fn invoke(&self, method: &str, payload: &Value) -> Result<Value, String> {
        match method {
            "info" => {
                let info = TransportInfo {
                    id: TRANSPORT_ID.to_string(),
                    name: "Matrix".to_string(),
                    role: "federation".to_string(),
                    trust: if self.configured() {
                        "configured".to_string()
                    } else {
                        "unconfigured".to_string()
                    },
                    latency: 15,
                };
                Ok(serde_json::to_value(&info).unwrap_or_default())
            }
            "health" => {
                let health = self.health_check()?;
                Ok(serde_json::to_value(&health).unwrap_or_default())
            }
            "list" => {
                if !self.configured() {
                    return Ok(envelope_ok(serde_json::json!([])));
                }
                match self.sync_messages() {
                    Ok(msgs) => Ok(envelope_ok(serde_json::to_value(&msgs).unwrap_or_default())),
                    Err(e) => Ok(envelope_err(&e)),
                }
            }
            "send" => {
                let text = payload.get("text").and_then(|v| v.as_str()).unwrap_or("");
                if text.is_empty() {
                    return Ok(envelope_ok(
                        serde_json::json!({"accepted": false, "reason": "empty"}),
                    ));
                }
                if !self.configured() {
                    return Ok(envelope_ok(serde_json::json!({
                        "accepted": false,
                        "reason": "matrix not configured"
                    })));
                }
                match self.send_message(text) {
                    Ok(resp) => Ok(envelope_ok(resp)),
                    Err(e) => Ok(envelope_err(&e)),
                }
            }
            _ => Err(format!("unknown matrix method: {method}")),
        }
    }
}

pub(crate) fn timestamp_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_unconfigured_health() {
        let client = MatrixClient {
            homeserver: None,
            user_id: None,
            access_token: None,
            room_id: None,
        };
        let result = client.invoke("health", &serde_json::json!({})).unwrap();
        let health: TransportHealth = serde_json::from_value(result).unwrap();
        assert!(!health.connected);
        assert!(health.last_error.is_some());
    }

    #[test]
    fn matrix_info() {
        let client = MatrixClient::current();
        let result = client.invoke("info", &serde_json::json!({})).unwrap();
        let info: TransportInfo = serde_json::from_value(result).unwrap();
        assert_eq!(info.id, "matrix");
        assert_eq!(info.role, "federation");
    }
}
