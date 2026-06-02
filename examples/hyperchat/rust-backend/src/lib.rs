//! HyperChat Rust backend — Matrix, Stalwart, and static Bitchat status exposed to Swift via
//! [eqswift](https://crates.io/crates/eqswift) (crates.io) and UniFFI.

mod matrix;
mod matrix_session;
mod stalwart;
mod transport_types;

eqswift::setup!();

fn json_result(result: Result<serde_json::Value, String>) -> String {
    match result {
        Ok(v) => serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string()),
        Err(e) => serde_json::json!({ "error": e }).to_string(),
    }
}

#[eqswift::export]
pub fn set_matrix_config(
    homeserver: Option<String>,
    user_id: Option<String>,
    access_token: Option<String>,
    room_id: Option<String>,
) {
    matrix::set_matrix_config(homeserver, user_id, access_token, room_id);
}

#[eqswift::export]
pub fn set_stalwart_config(base_url: String, username: Option<String>, password: Option<String>) {
    stalwart::set_stalwart_config(base_url, username, password);
}

/// Matrix Client-Server health snapshot as JSON (`MATRIX_*` env).
#[eqswift::export]
pub fn matrix_health_json() -> String {
    let c = matrix::MatrixClient::current();
    json_result(c.invoke("health", &serde_json::json!({})))
}

/// Send plain text to the configured Matrix room (`MATRIX_ROOM_ID`).
#[eqswift::export]
pub fn matrix_send_json(text: String) -> String {
    let c = matrix::MatrixClient::current();
    json_result(c.invoke("send", &serde_json::json!({ "text": text })))
}

/// Stalwart JMAP health snapshot (`STALWART_*` env).
#[eqswift::export]
pub fn stalwart_health_json() -> String {
    let c = stalwart::StalwartClient::current();
    json_result(c.invoke("health", &serde_json::json!({})))
}

/// Archive a short text payload via Stalwart JMAP (`STALWART_*` env).
#[eqswift::export]
pub fn stalwart_send_json(text: String) -> String {
    let c = stalwart::StalwartClient::current();
    json_result(c.invoke("send", &serde_json::json!({ "text": text })))
}

#[eqswift::export]
pub fn stalwart_list_mailboxes_json() -> String {
    let c = stalwart::StalwartClient::current();
    json_result(c.invoke("list_mailboxes", &serde_json::json!({})))
}

#[eqswift::export]
pub fn stalwart_list_emails_json(mailbox_id: String, since: Option<String>) -> String {
    let c = stalwart::StalwartClient::current();
    json_result(c.invoke(
        "list_emails",
        &serde_json::json!({ "mailbox_id": mailbox_id, "since": since }),
    ))
}

/// Bitchat: upstream SwiftPM is executable-only — static status JSON for UI.
#[eqswift::export]
pub fn bitchat_status_json() -> String {
    serde_json::json!({
        "id": "bitchat",
        "name": "Bitchat",
        "role": "mesh",
        "connected": false,
        "latency_ms": 0,
        "last_error": "permissionlesstech/bitchat exposes an executable product only — no Swift library to link yet."
    })
    .to_string()
}

/// Reload Matrix / Stalwart clients from process environment (after Settings save).
#[eqswift::export]
pub fn reload_transports() {
    matrix_session::reload_transports();
}

/// Joined Matrix rooms (`/_matrix/client/v3/joined_rooms`).
#[eqswift::export]
pub fn matrix_joined_rooms_json() -> String {
    json_result(matrix_session::joined_rooms_json())
}

/// Incremental `/sync` using stored `since` token (updates token on success).
#[eqswift::export]
pub fn matrix_sync_delta_json() -> String {
    json_result(matrix_session::sync_delta_json())
}

/// Send plain text into an arbitrary room id.
#[eqswift::export]
pub fn matrix_send_room_json(room_id: String, text: String) -> String {
    json_result(matrix_session::send_room_json(room_id, text))
}

#[eqswift::export]
pub fn matrix_typing_json(room_id: String, typing: bool) -> String {
    json_result(matrix_session::typing_json(room_id, typing))
}

#[eqswift::export]
pub fn matrix_upload_media_json(
    room_id: String,
    filename: String,
    mime: String,
    data_base64: String,
) -> String {
    json_result(matrix_session::upload_media_json(
        room_id,
        filename,
        mime,
        data_base64,
    ))
}
