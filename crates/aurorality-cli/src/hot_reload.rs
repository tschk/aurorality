//! WebSocket hot-reload server for `aurorality dev`.
//!
//! Speaks the `HotReloadEnvelope` protocol from `crepuscularity-native` that
//! `HotReloadClient` (Swift) decodes: `devHello` on connect, then the current
//! IR as `fullReload`, then `patch` / `fullReload` / `error` per `.crepus` save,
//! plus `swiftgenStatus` when swiftgen is configured.

use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crepuscularity_core::context::TemplateContext;
use crepuscularity_native::{
    plan_hot_reload, render_template_to_ir, HotReloadEnvelope, HotReloadMessage,
};
use tungstenite::{Message, WebSocket};

pub const DEFAULT_PORT: u16 = 47832;

#[derive(Clone)]
pub struct SwiftgenConfig {
    pub view: PathBuf,
    pub out: PathBuf,
    pub view_name: String,
    pub context_type: String,
}

pub struct ServerConfig {
    pub watch_dir: PathBuf,
    pub ir_enabled: bool,
    pub swiftgen: Option<SwiftgenConfig>,
}

pub struct HotReloadServer {
    cfg: ServerConfig,
    session_id: String,
    seq: AtomicU64,
    clients: Mutex<Vec<WebSocket<TcpStream>>>,
    /// Last seen contents of each template, and which one clients are showing.
    templates: Mutex<(HashMap<PathBuf, String>, Option<PathBuf>)>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl HotReloadServer {
    /// Load the templates in `cfg.watch_dir` and accept clients on `listener`
    /// in a background thread.
    pub fn start(listener: TcpListener, cfg: ServerConfig) -> Arc<Self> {
        let mut templates = HashMap::new();
        let mut paths: Vec<PathBuf> = std::fs::read_dir(&cfg.watch_dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "crepus"))
            .collect();
        paths.sort();
        for p in &paths {
            if let Ok(text) = std::fs::read_to_string(p) {
                templates.insert(p.clone(), text);
            }
        }
        let active = paths
            .iter()
            .find(|p| p.file_stem().is_some_and(|s| s == "main"))
            .or(paths.first())
            .cloned();

        let server = Arc::new(Self {
            cfg,
            session_id: format!("{:x}-{}", now_ms(), std::process::id()),
            // Clients drop envelopes whose sequence is not above the last one they
            // saw; seeding from the clock keeps a restarted server ahead.
            seq: AtomicU64::new(now_ms()),
            clients: Mutex::new(Vec::new()),
            templates: Mutex::new((templates, active)),
        });
        let accept = Arc::clone(&server);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let server = Arc::clone(&accept);
                std::thread::spawn(move || server.accept_client(stream));
            }
        });
        server
    }

    pub fn client_count(&self) -> usize {
        self.clients.lock().map(|c| c.len()).unwrap_or(0)
    }

    fn envelope(&self, message: HotReloadMessage) -> Message {
        let envelope = HotReloadEnvelope {
            sequence: self.seq.fetch_add(1, Ordering::SeqCst) + 1,
            message,
        };
        Message::text(serde_json::to_string(&envelope).unwrap_or_default())
    }

    fn accept_client(&self, stream: TcpStream) {
        let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
        let Ok(mut ws) = tungstenite::accept(stream) else {
            return;
        };
        let swiftgen = self.cfg.swiftgen.as_ref();
        let hello = HotReloadMessage::DevHello {
            session_id: self.session_id.clone(),
            watch_dir: self.cfg.watch_dir.display().to_string(),
            swiftgen_view: swiftgen.map(|s| s.view.display().to_string()),
            swiftgen_out: swiftgen.map(|s| s.out.display().to_string()),
            swiftgen_view_name: swiftgen.map(|s| s.view_name.clone()),
            swiftgen_context_type: swiftgen.map(|s| s.context_type.clone()),
            ir_enabled: self.cfg.ir_enabled,
        };
        if ws.send(self.envelope(hello)).is_err() {
            return;
        }
        if self.cfg.ir_enabled {
            let current = {
                let guard = self.templates.lock().unwrap();
                guard.1.as_ref().and_then(|p| guard.0.get(p)).cloned()
            };
            if let Some(template) = current {
                let msg = full_reload(&template, "connected");
                if ws.send(self.envelope(msg)).is_err() {
                    return;
                }
            }
        }
        self.clients.lock().unwrap().push(ws);
    }

    fn broadcast(&self, message: HotReloadMessage) {
        let msg = self.envelope(message);
        let mut clients = self.clients.lock().unwrap();
        clients.retain_mut(|ws| ws.send(msg.clone()).is_ok());
    }

    /// Re-read a saved `.crepus` file and push the resulting update to clients.
    pub fn template_changed(&self, path: &Path) {
        let Ok(new) = std::fs::read_to_string(path) else {
            return;
        };
        if self.cfg.ir_enabled {
            let message = {
                let mut guard = self.templates.lock().unwrap();
                let (templates, active) = &mut *guard;
                let old = templates.insert(path.to_path_buf(), new.clone());
                let same_view = active.as_deref() == Some(path);
                *active = Some(path.to_path_buf());
                match old {
                    // Patches are relative to what the client shows, so only diff
                    // against the template that is already on screen.
                    Some(old) if same_view => plan_hot_reload(&old, &new, &TemplateContext::new()),
                    _ => full_reload(&new, "template switched"),
                }
            };
            if !matches!(message, HotReloadMessage::Noop) {
                self.broadcast(message);
            }
        }
        if let Some(sg) = &self.cfg.swiftgen {
            if same_file(&sg.view, path) {
                let result =
                    crate::swiftgen::run(&sg.view, &sg.out, &sg.view_name, &sg.context_type);
                self.broadcast(HotReloadMessage::SwiftgenStatus {
                    ok: result.is_ok(),
                    errors: result
                        .err()
                        .map(|e| vec![format!("{e:#}")])
                        .unwrap_or_default(),
                    view_name: sg.view_name.clone(),
                    output_path: sg
                        .out
                        .join(format!("{}.swift", sg.view_name))
                        .display()
                        .to_string(),
                    ts_ms: now_ms(),
                });
            }
        }
    }
}

fn full_reload(template: &str, reason: &str) -> HotReloadMessage {
    match render_template_to_ir(template, &TemplateContext::new()) {
        Ok(ir) => HotReloadMessage::FullReload {
            ir,
            reason: reason.to_string(),
        },
        Err(message) => HotReloadMessage::Error { message },
    }
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn recv(ws: &mut WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>) -> Value {
        let msg = ws.read().unwrap();
        serde_json::from_str(msg.to_text().unwrap()).unwrap()
    }

    #[test]
    fn serves_hello_current_ir_and_updates_on_save() {
        let dir = tempfile::tempdir().unwrap();
        let main = dir.path().join("main.crepus");
        std::fs::write(&main, "span text-base\n  \"first\"\n").unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = HotReloadServer::start(
            listener,
            ServerConfig {
                watch_dir: dir.path().to_path_buf(),
                ir_enabled: true,
                swiftgen: None,
            },
        );

        let (mut ws, _) = tungstenite::connect(format!("ws://127.0.0.1:{port}")).unwrap();
        let hello = recv(&mut ws);
        assert_eq!(hello["message"]["kind"], "devHello");
        assert_eq!(hello["message"]["ir_enabled"], true);
        let first = recv(&mut ws);
        assert_eq!(first["message"]["kind"], "fullReload");
        assert!(
            first["message"]["ir"].to_string().contains("first"),
            "{first}"
        );
        assert!(first["sequence"].as_u64() > hello["sequence"].as_u64());

        // Wait until the accept thread has registered the client.
        for _ in 0..100 {
            if server.client_count() == 1 {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        std::fs::write(&main, "span text-base\n  \"second\"\n").unwrap();
        server.template_changed(&main);
        let update = recv(&mut ws);
        let kind = update["message"]["kind"].as_str().unwrap();
        assert!(matches!(kind, "patch" | "fullReload"), "{update}");
        assert!(update.to_string().contains("second"), "{update}");
        assert!(!update.to_string().contains("first"), "{update}");
    }
}
