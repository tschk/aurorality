//! Process-wide Matrix session (sync cursor + joined rooms + room-scoped send).

use std::sync::{Mutex, OnceLock};

use serde_json::{json, Value};

use crate::matrix::MatrixClient;
use crate::transport_types::{envelope_err, envelope_ok};

struct MatrixSessionState {
    client: MatrixClient,
    since: Option<String>,
}

impl MatrixSessionState {
    fn new() -> Self {
        Self {
            client: MatrixClient::from_env(),
            since: None,
        }
    }
}

fn state() -> &'static Mutex<MatrixSessionState> {
    static S: OnceLock<Mutex<MatrixSessionState>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(MatrixSessionState::new()))
}

pub fn reload_from_env() {
    let mut g = state().lock().expect("matrix session lock");
    *g = MatrixSessionState::new();
}

pub fn joined_rooms_json() -> Result<Value, String> {
    let st = state().lock().expect("matrix session lock");
    if !st.client.token_ready() {
        return Ok(json!({ "joined_rooms": [] }));
    }
    let hs = st.client.homeserver_trimmed()?;
    let url = format!("{hs}/_matrix/client/v3/joined_rooms");
    st.client.auth_get(&url)
}

pub fn sync_delta_json() -> Result<Value, String> {
    let mut st = state().lock().expect("matrix session lock");
    if !st.client.token_ready() {
        return Err("matrix not configured".into());
    }
    let hs = st.client.homeserver_trimmed()?;
    let mut url = format!("{hs}/_matrix/client/v3/sync?timeout=30000&full_state=false");
    if let Some(ref s) = st.since {
        url.push_str("&since=");
        url.push_str(s);
    }
    let v = st.client.auth_get(&url)?;
    if let Some(nb) = v.get("next_batch").and_then(|x| x.as_str()) {
        st.since = Some(nb.to_string());
    }
    Ok(v)
}

pub fn send_room_json(room_id: String, text: String) -> Result<Value, String> {
    let st = state().lock().expect("matrix session lock");
    if !st.client.token_ready() {
        return Ok(envelope_ok(
            json!({"accepted": false, "reason": "matrix not configured"}),
        ));
    }
    let hs = st.client.homeserver_trimmed()?;
    let txn = format!("hc{}", crate::matrix::timestamp_ms());
    let url = format!(
        "{hs}/_matrix/client/v3/rooms/{}/send/m.room.message/{txn}",
        encode_matrix_path(&room_id)
    );
    let body = json!({
        "msgtype": "m.text",
        "body": text,
    });
    let resp = st.client.auth_put(&url, body)?;
    Ok(envelope_ok(resp))
}

pub fn typing_json(room_id: String, typing: bool) -> Result<Value, String> {
    let st = state().lock().expect("matrix session lock");
    let Some(uid) = st.client.user_id_owned() else {
        return Ok(envelope_ok(json!({"ok": false, "reason": "no user id"})));
    };
    if !st.client.token_ready() {
        return Ok(envelope_ok(json!({"ok": false})));
    }
    let hs = st.client.homeserver_trimmed()?;
    let url = format!(
        "{hs}/_matrix/client/v3/rooms/{}/typing/{}",
        encode_matrix_path(&room_id),
        encode_matrix_path(&uid)
    );
    let body = json!({ "typing": typing, "timeout": 30000u32 });
    let _ = st.client.auth_put(&url, body)?;
    Ok(envelope_ok(json!({"ok": true})))
}

pub fn upload_media_json(
    room_id: String,
    filename: String,
    mime: String,
    data_base64: String,
) -> Result<Value, String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data_base64.trim())
        .map_err(|e| format!("base64: {e}"))?;
    let st = state().lock().expect("matrix session lock");
    if !st.client.token_ready() {
        return Ok(envelope_err("matrix not configured"));
    }
    let hs = st.client.homeserver_trimmed()?;
    let upload_url = format!(
        "{hs}/_matrix/media/v3/upload?filename={}",
        encode_query_value(&filename)
    );
    let mxc = st.client.upload_media_raw(&upload_url, &mime, &bytes)?;
    let txn = format!("hc{}", crate::matrix::timestamp_ms());
    let send_url = format!(
        "{hs}/_matrix/client/v3/rooms/{}/send/m.room.message/{txn}",
        encode_matrix_path(&room_id)
    );
    let body = json!({
        "msgtype": "m.image",
        "body": filename,
        "url": mxc,
    });
    let resp = st.client.auth_put(&send_url, body)?;
    Ok(envelope_ok(resp))
}

fn encode_matrix_path(s: &str) -> String {
    let mut out = String::new();
    for b in s.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(*b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn encode_query_value(s: &str) -> String {
    encode_matrix_path(s)
}
