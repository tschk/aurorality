//! File watcher + WebSocket broadcast server for hot reload.
//!
//! Watches a directory of `.crepus` files. When a file changes it re-renders
//! the template and broadcasts a [`crepuscularity_native::HotReloadEnvelope`]
//! to all connected clients (the AurorRunner preview window or your app).
//!
//! Optionally runs [`crate::swiftgen`] after each save and broadcasts
//! [`HotReloadMessage::SwiftgenStatus`] for hybrid compile-time + IR workflows.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode, DebounceEventResult};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;

use crepuscularity_core::context::TemplateContext;
use crepuscularity_native::{plan_hot_reload, HotReloadEnvelope, HotReloadMessage};

#[derive(Clone, Debug)]
pub struct SwiftgenDevConfig {
    pub view: PathBuf,
    pub out: PathBuf,
    pub view_name: String,
    pub context_type: String,
}

#[derive(Clone, Debug)]
pub struct DevServerConfig {
    pub watch_dir: PathBuf,
    pub port: u16,
    pub swiftgen: Option<SwiftgenDevConfig>,
    pub ir_enabled: bool,
}

pub async fn run(cfg: DevServerConfig) -> Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], cfg.port));
    let listener = TcpListener::bind(&addr).await?;

    println!("aurorality dev  →  ready on port {}", cfg.port);
    println!("watching        →  {}", cfg.watch_dir.display());
    if let Some(sg) = &cfg.swiftgen {
        println!(
            "swiftgen        →  {}  →  {}",
            sg.view.display(),
            sg.out.display()
        );
    }
    println!(
        "IR updates      →  {}",
        if cfg.ir_enabled {
            "on"
        } else {
            "off (--no-ir)"
        }
    );

    let session_id = format!(
        "{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );

    let (tx, _rx) = broadcast::channel::<String>(64);
    let tx = Arc::new(tx);

    let file_cache: Arc<tokio::sync::Mutex<HashMap<PathBuf, String>>> =
        Arc::new(tokio::sync::Mutex::new(load_all_crepus(&cfg.watch_dir)));

    let sequence = Arc::new(AtomicU64::new(0));

    {
        let tx = tx.clone();
        let cache = file_cache.clone();
        let seq = sequence.clone();
        let dir = cfg.watch_dir.clone();
        let swiftgen = cfg.swiftgen.clone();
        let ir_enabled = cfg.ir_enabled;
        tokio::task::spawn_blocking(move || {
            watch_files(dir, tx, cache, seq, swiftgen, ir_enabled);
        });
    }

    let watch_dir = cfg.watch_dir.clone();
    let swiftgen_cfg = cfg.swiftgen.clone();
    let ir_enabled = cfg.ir_enabled;
    let session_for_hello = session_id;

    while let Ok((stream, peer)) = listener.accept().await {
        println!("connected  →  {peer}");
        let rx = tx.subscribe();

        let snapshot = if ir_enabled {
            snapshot_ir(&file_cache, &watch_dir).await
        } else {
            None
        };

        let hello = hello_envelope(
            &sequence,
            &session_for_hello,
            &watch_dir,
            &swiftgen_cfg,
            ir_enabled,
        );

        tokio::spawn(handle_connection(stream, peer, rx, hello, snapshot));
    }

    Ok(())
}

fn hello_envelope(
    sequence: &Arc<AtomicU64>,
    session_id: &str,
    watch_dir: &Path,
    swiftgen: &Option<SwiftgenDevConfig>,
    ir_enabled: bool,
) -> Option<String> {
    let seq = sequence.fetch_add(1, Ordering::SeqCst) + 1;
    let (swiftgen_view, swiftgen_out, swiftgen_view_name, swiftgen_context_type) =
        if let Some(sg) = swiftgen {
            (
                Some(sg.view.display().to_string()),
                Some(sg.out.display().to_string()),
                Some(sg.view_name.clone()),
                Some(sg.context_type.clone()),
            )
        } else {
            (None, None, None, None)
        };
    let msg = HotReloadMessage::DevHello {
        session_id: session_id.to_string(),
        watch_dir: watch_dir.display().to_string(),
        swiftgen_view,
        swiftgen_out,
        swiftgen_view_name,
        swiftgen_context_type,
        ir_enabled,
    };
    let env = HotReloadEnvelope {
        sequence: seq,
        message: msg,
    };
    serde_json::to_string(&env).ok()
}

async fn snapshot_ir(
    cache: &Arc<tokio::sync::Mutex<HashMap<PathBuf, String>>>,
    _watch_dir: &Path,
) -> Option<String> {
    let cache = cache.lock().await;
    let (path, content) = cache.iter().next()?;
    let ctx = TemplateContext::new();
    let ir = crepuscularity_native::render_template_to_ir(content, &ctx).ok()?;
    let msg = HotReloadMessage::FullReload {
        ir,
        reason: format!("initial load of {}", path.display()),
    };
    let env = HotReloadEnvelope {
        sequence: 0,
        message: msg,
    };
    serde_json::to_string(&env).ok()
}

async fn handle_connection(
    stream: TcpStream,
    peer: SocketAddr,
    mut rx: broadcast::Receiver<String>,
    hello: Option<String>,
    initial_ir: Option<String>,
) {
    let ws = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("WS handshake failed for {peer}: {e}");
            return;
        }
    };
    let (mut sink, mut source) = ws.split();

    if let Some(h) = hello {
        let _ = sink.send(Message::Text(h)).await;
    }
    if let Some(snapshot) = initial_ir {
        let _ = sink.send(Message::Text(snapshot)).await;
    }

    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(json) => {
                        if sink.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
            incoming = source.next() => {
                if incoming.is_none() { break; }
            }
        }
    }
    println!("disconnected  →  {peer}");
}

fn watch_files(
    watch_dir: PathBuf,
    tx: Arc<broadcast::Sender<String>>,
    cache: Arc<tokio::sync::Mutex<HashMap<PathBuf, String>>>,
    sequence: Arc<AtomicU64>,
    swiftgen: Option<SwiftgenDevConfig>,
    ir_enabled: bool,
) {
    let rt = tokio::runtime::Handle::current();
    let (notify_tx, notify_rx) = std::sync::mpsc::channel::<DebounceEventResult>();
    let mut debouncer =
        new_debouncer(Duration::from_millis(150), notify_tx).expect("debouncer init");

    if !watch_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&watch_dir) {
            eprintln!("watch: could not create {} ({e})", watch_dir.display());
            return;
        }
        eprintln!("watch: created {}", watch_dir.display());
    }

    debouncer
        .watcher()
        .watch(&watch_dir, RecursiveMode::Recursive)
        .expect("watch dir");

    let swiftgen_view_canon = swiftgen
        .as_ref()
        .and_then(|s| std::fs::canonicalize(&s.view).ok());

    for result in notify_rx {
        let events = match result {
            Ok(events) => events,
            Err(error) => {
                eprintln!("watch error: {error}");
                continue;
            }
        };

        for event in events {
            let path = &event.path;
            if path.extension().map_or(true, |e| e != "crepus") {
                continue;
            }

            let new_content = match std::fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("read error {}: {e}", path.display());
                    continue;
                }
            };

            let path = path.clone();
            let tx = tx.clone();
            let cache = cache.clone();
            let sequence = sequence.clone();
            let swiftgen = swiftgen.clone();
            let sg_canon = swiftgen_view_canon.clone();
            let ir_enabled = ir_enabled;

            rt.spawn(async move {
                let old_content = {
                    let mut map = cache.lock().await;
                    let old = map.get(&path).cloned().unwrap_or_default();
                    map.insert(path.clone(), new_content.clone());
                    old
                };

                let ctx = TemplateContext::new();

                if ir_enabled {
                    let seq = sequence.fetch_add(1, Ordering::SeqCst) + 1;
                    let msg = plan_hot_reload(&old_content, &new_content, &ctx);
                    let env = HotReloadEnvelope {
                        sequence: seq,
                        message: msg,
                    };
                    if let Ok(json) = serde_json::to_string(&env) {
                        let _ = tx.send(json);
                    }
                }

                let path_canon = std::fs::canonicalize(&path).unwrap_or(path.clone());
                let run_sg = swiftgen.as_ref().map_or(false, |_cfg| {
                    sg_canon.as_ref().map(|c| c == &path_canon).unwrap_or(false)
                });

                if run_sg {
                    if let Some(cfg) = swiftgen {
                        let ts_ms = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;
                        let result = crate::swiftgen::run(
                            &cfg.view,
                            &cfg.out,
                            &cfg.view_name,
                            &cfg.context_type,
                        );
                        let seq = sequence.fetch_add(1, Ordering::SeqCst) + 1;
                        let (ok, errors, output_path) = match result {
                            Ok(()) => (
                                true,
                                vec![],
                                cfg.out
                                    .join(format!("{}.swift", cfg.view_name))
                                    .display()
                                    .to_string(),
                            ),
                            Err(e) => {
                                let errs: Vec<String> = e.chain().map(|c| c.to_string()).collect();
                                (
                                    false,
                                    if errs.is_empty() {
                                        vec![e.to_string()]
                                    } else {
                                        errs
                                    },
                                    cfg.out
                                        .join(format!("{}.swift", cfg.view_name))
                                        .display()
                                        .to_string(),
                                )
                            }
                        };
                        let msg = HotReloadMessage::SwiftgenStatus {
                            ok,
                            errors,
                            view_name: cfg.view_name.clone(),
                            output_path,
                            ts_ms,
                        };
                        let env = HotReloadEnvelope {
                            sequence: seq,
                            message: msg,
                        };
                        if let Ok(json) = serde_json::to_string(&env) {
                            let _ = tx.send(json);
                        }
                    }
                }
            });
        }
    }
}

fn load_all_crepus(dir: &Path) -> HashMap<PathBuf, String> {
    let mut map = HashMap::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return map;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "crepus") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                map.insert(path, content);
            }
        }
    }
    map
}
