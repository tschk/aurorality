//! File watcher + WebSocket broadcast server for hot reload.
//!
//! Watches a directory of `.crepus` files. When a file changes it re-renders
//! the template and broadcasts a [`crepuscularity_native::HotReloadEnvelope`]
//! to all connected WebSocket clients (the Runner app).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode, DebounceEventResult};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;

use crepuscularity_core::context::TemplateContext;
use crepuscularity_native::{plan_hot_reload, HotReloadEnvelope, HotReloadMessage};

/// Start the dev server: file watcher + WebSocket broadcast.
///
/// - `watch_dir`: directory of `.crepus` files to monitor
/// - `port`: local TCP port (default `47832`)
pub async fn run(watch_dir: PathBuf, port: u16) -> Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(&addr).await?;
    println!("aurorality dev  →  ws://{addr}");
    println!("watching        →  {}", watch_dir.display());

    // Channel for broadcasting IR update messages to all connected WS clients.
    let (tx, _rx) = broadcast::channel::<String>(32);
    let tx = Arc::new(tx);

    // Seed the file cache with the current state of every .crepus file.
    let file_cache: Arc<tokio::sync::Mutex<HashMap<PathBuf, String>>> =
        Arc::new(tokio::sync::Mutex::new(load_all_crepus(&watch_dir)));

    let sequence = Arc::new(AtomicU64::new(0));

    // Spawn the file watcher on a blocking thread.
    {
        let tx = tx.clone();
        let cache = file_cache.clone();
        let seq = sequence.clone();
        let dir = watch_dir.clone();
        tokio::task::spawn_blocking(move || {
            watch_files(dir, tx, cache, seq);
        });
    }

    // Accept WebSocket connections.
    while let Ok((stream, peer)) = listener.accept().await {
        println!("runner connected  →  {peer}");
        let rx = tx.subscribe();
        // Send the current full state to the newly connected client.
        let snapshot = snapshot_ir(&file_cache, &watch_dir).await;
        tokio::spawn(handle_connection(stream, peer, rx, snapshot));
    }

    Ok(())
}

async fn snapshot_ir(
    cache: &Arc<tokio::sync::Mutex<HashMap<PathBuf, String>>>,
    _watch_dir: &Path,
) -> Option<String> {
    let cache = cache.lock().await;
    // For the initial snapshot we combine all files by rendering each and
    // wrapping in a FullReload envelope using the first file we find.
    let (path, content) = cache.iter().next()?;
    let ctx = TemplateContext::new();
    let ir = crepuscularity_native::render_template_to_ir(content, &ctx).ok()?;
    let msg = HotReloadMessage::FullReload {
        ir,
        reason: format!("initial load of {}", path.display()),
    };
    let env = HotReloadEnvelope { sequence: 0, message: msg };
    serde_json::to_string(&env).ok()
}

async fn handle_connection(
    stream: TcpStream,
    peer: SocketAddr,
    mut rx: broadcast::Receiver<String>,
    initial: Option<String>,
) {
    let ws = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("WS handshake failed for {peer}: {e}");
            return;
        }
    };
    let (mut sink, mut source) = ws.split();

    // Send current state immediately on connect.
    if let Some(snapshot) = initial {
        let _ = sink.send(Message::Text(snapshot.into())).await;
    }

    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(json) => {
                        if sink.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
            // Drain incoming messages (ping/pong handled by tungstenite).
            incoming = source.next() => {
                if incoming.is_none() { break; }
            }
        }
    }
    println!("runner disconnected  →  {peer}");
}

fn watch_files(
    watch_dir: PathBuf,
    tx: Arc<broadcast::Sender<String>>,
    cache: Arc<tokio::sync::Mutex<HashMap<PathBuf, String>>>,
    sequence: Arc<AtomicU64>,
) {
    let rt = tokio::runtime::Handle::current();
    let (notify_tx, notify_rx) = std::sync::mpsc::channel::<DebounceEventResult>();
    let mut debouncer =
        new_debouncer(Duration::from_millis(150), notify_tx).expect("debouncer init");
    debouncer
        .watcher()
        .watch(&watch_dir, RecursiveMode::Recursive)
        .expect("watch dir");

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

            let seq = sequence.fetch_add(1, Ordering::SeqCst) + 1;
            let tx = tx.clone();
            let cache = cache.clone();
            let path = path.clone();

            rt.spawn(async move {
                let old_content = {
                    let mut map = cache.lock().await;
                    let old = map.get(&path).cloned().unwrap_or_default();
                    map.insert(path.clone(), new_content.clone());
                    old
                };

                let ctx = TemplateContext::new();
                let msg = plan_hot_reload(&old_content, &new_content, &ctx);
                let env = HotReloadEnvelope { sequence: seq, message: msg };
                match serde_json::to_string(&env) {
                    Ok(json) => {
                        let _ = tx.send(json);
                    }
                    Err(e) => eprintln!("serialise error: {e}"),
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
        if path.extension().map_or(false, |e| e == "crepus") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                map.insert(path, content);
            }
        }
    }
    map
}
