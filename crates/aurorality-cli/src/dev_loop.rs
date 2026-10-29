//! `aurorality dev` — watch → live reload / rebuild → relaunch loop.
//!
//! The app is launched with `AURORALITY_VIEWS_DIR` (the live `views/` dir) and
//! `AURORALITY_LIVE_MARKER` (a file it creates when it watches its templates
//! itself, see `AurorState.watchLiveTemplate`). A `.crepus` save then needs no
//! action here; no port is opened. With `--serve`, saves are also pushed over
//! WebSocket (see [`crate::hot_reload`]). Anything else — Swift changes, or a
//! template save no running app picks up — kills the app, rebuilds with
//! `swift build`, and relaunches it.

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecursiveMode, Watcher};

use crate::build_swift;
use crate::hot_reload::{HotReloadServer, ServerConfig, SwiftgenConfig};

/// `dev --serve`: also run the WebSocket hot-reload server.
pub struct Serve {
    pub port: u16,
    pub ir_enabled: bool,
    pub swiftgen: Option<SwiftgenConfig>,
}

/// Environment handed to the launched app.
struct Launch {
    views_dir: PathBuf,
    marker: PathBuf,
    port: Option<u16>,
}

/// Build, launch, then watch → (live reload) / rebuild → relaunch.
pub fn run(watch_views: PathBuf, serve: Option<Serve>) -> anyhow::Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Register the watcher before serving or building, so a save made as soon as
    // a client connects (or during the first build) is not lost.
    let cfg = build_swift::read_config(&cwd).ok();
    let watched_sources = watched_source_dirs(&cwd, cfg.as_ref());
    let watched_resources = watched_resource_dirs(&cwd, &watch_views, cfg.as_ref());
    println!(
        "{} watching {} — edit to rebuild",
        console::style("aurorality dev").cyan().bold(),
        watched_description(&watched_sources, &watched_resources),
    );
    // File watcher
    let (tx, rx) = std::sync::mpsc::channel::<PathBuf>();
    let tx_notify = tx.clone();

    let mut watcher = match notify::recommended_watcher(move |res: notify::Result<Event>| {
        if let Ok(ev) = res {
            match ev.kind {
                EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
                    if let Some(path) = ev.paths.into_iter().next() {
                        let _ = tx_notify.send(path);
                    }
                }
                _ => {}
            }
        }
    }) {
        Ok(w) => w,
        Err(e) => anyhow::bail!("could not create file watcher: {e}"),
    };

    for path in &watched_resources {
        watcher.watch(path, RecursiveMode::Recursive).ok();
    }
    for path in &watched_sources {
        watcher.watch(path, RecursiveMode::Recursive).ok();
    }
    // Watch Package.swift
    let pkg = cwd.join("Package.swift");
    if pkg.exists() {
        watcher.watch(&pkg, RecursiveMode::NonRecursive).ok();
    }

    let server = match &serve {
        Some(s) => {
            let port = s.port;
            let listener = std::net::TcpListener::bind(("127.0.0.1", port))
                .map_err(|e| anyhow::anyhow!("hot-reload server: bind 127.0.0.1:{port}: {e}"))?;
            println!("hot reload on ws://127.0.0.1:{port}");
            Some(HotReloadServer::start(
                listener,
                ServerConfig {
                    watch_dir: cwd.join(&watch_views),
                    ir_enabled: s.ir_enabled,
                    swiftgen: s.swiftgen.clone(),
                },
            ))
        }
        None => None,
    };
    let launch = Launch {
        views_dir: std::path::absolute(cwd.join(&watch_views))?,
        marker: std::path::absolute(
            cwd.join(".build")
                .join(format!("aurorality-live-{}", std::process::id())),
        )?,
        port: serve.as_ref().map(|s| s.port),
    };

    // Pre-build Rust + bindings (only once)
    crate::pre_build().map_err(|e| anyhow::anyhow!("pre-build failed: {e}"))?;

    let child: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));
    let shutdown = Arc::new(AtomicBool::new(false));
    let child_for_signal = Arc::clone(&child);
    let shutdown_for_signal = Arc::clone(&shutdown);
    if let Err(e) = ctrlc::set_handler(move || {
        shutdown_for_signal.store(true, Ordering::Relaxed);
        stop_running_app(&child_for_signal, false);
    }) {
        eprintln!("could not install Ctrl-C handler: {e}");
    }

    // Initial build + launch
    {
        let mut child = child.lock().unwrap();
        *child = build_and_launch_app(&cwd, &launch);
    }

    loop {
        if shutdown.load(Ordering::Relaxed) {
            break;
        }

        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(first) => {
                // Debounce: collect changes for 300ms
                let mut changed = vec![first];
                let t = Instant::now();
                while t.elapsed() < Duration::from_millis(300) {
                    while let Ok(p) = rx.try_recv() {
                        changed.push(p);
                    }
                    std::thread::sleep(Duration::from_millis(30));
                }
                changed.sort();
                changed.dedup();

                let templates_only = changed
                    .iter()
                    .all(|p| p.extension().is_some_and(|e| e == "crepus"));
                let mut handled = false;
                if let Some(server) = &server {
                    for p in changed
                        .iter()
                        .filter(|p| p.extension().is_some_and(|e| e == "crepus"))
                    {
                        server.template_changed(p);
                    }
                    // With --no-ir the server pushes no template updates.
                    let ir = serve.as_ref().is_some_and(|s| s.ir_enabled);
                    if templates_only && ir && server.client_count() > 0 {
                        println!(
                            "  pushed template update to {} client(s)",
                            server.client_count()
                        );
                        handled = true;
                    }
                }
                if templates_only && launch.marker.exists() && app_running(&child) {
                    println!("  template saved — app reloaded it in place");
                    handled = true;
                }
                if handled {
                    continue;
                }

                stop_running_app(&child, true);

                // Rebuild + relaunch
                let mut new = child.lock().unwrap();
                *new = build_and_launch_app(&cwd, &launch);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    stop_running_app(&child, false);
    let _ = std::fs::remove_file(&launch.marker);
    Ok(())
}

fn app_running(child: &Arc<Mutex<Option<Child>>>) -> bool {
    child
        .lock()
        .ok()
        .and_then(|mut c| c.as_mut().map(|c| matches!(c.try_wait(), Ok(None))))
        .unwrap_or(false)
}

fn build_and_launch_app(project_root: &Path, launch: &Launch) -> Option<Child> {
    let swift = crate::find_swift();
    let t0 = Instant::now();

    let status = Command::new(&swift)
        .arg("build")
        .current_dir(project_root)
        .status();

    match status {
        Ok(s) if s.success() => {
            let elapsed = t0.elapsed().as_millis();
            println!("  built in {elapsed} ms — launching");

            // A fresh launch must re-announce that it watches its templates.
            let _ = std::fs::remove_file(&launch.marker);
            let mut env = vec![
                (
                    "AURORALITY_VIEWS_DIR",
                    launch.views_dir.display().to_string(),
                ),
                (
                    "AURORALITY_LIVE_MARKER",
                    launch.marker.display().to_string(),
                ),
            ];
            if let Some(port) = launch.port {
                env.push(("AURORALITY_DEV_PORT", port.to_string()));
            }
            match build_swift::build_and_launch_spawn(project_root, &env) {
                Ok(child) => Some(child),
                Err(e) => {
                    eprintln!("  launch failed: {e}");
                    None
                }
            }
        }
        Ok(s) => {
            eprintln!("  build failed ({})", s.code().unwrap_or(1));
            None
        }
        Err(e) => {
            eprintln!("  swift build error: {e}");
            None
        }
    }
}

/// Stop only the app process this command started (never match by name: other
/// processes on the machine may share the project name).
fn stop_running_app(child: &Arc<Mutex<Option<Child>>>, restarting: bool) {
    if let Ok(mut old) = child.lock() {
        if let Some(mut c) = old.take() {
            if restarting {
                println!("  restarting…");
            }
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

fn watched_source_dirs(cwd: &Path, cfg: Option<&build_swift::ProjectConfig>) -> Vec<PathBuf> {
    let configured = cfg
        .and_then(|cfg| cfg.sources.as_ref())
        .map(|path| cwd.join(path));
    let mut dirs = Vec::new();
    if let Some(path) = configured {
        if path.exists() {
            dirs.push(path);
        }
    }
    let default_sources = cwd.join("Sources");
    if default_sources.exists() && !dirs.iter().any(|path| path == &default_sources) {
        dirs.push(default_sources);
    }
    dirs
}

fn watched_resource_dirs(
    cwd: &Path,
    watch_views: &Path,
    cfg: Option<&build_swift::ProjectConfig>,
) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if watch_views.exists() {
        dirs.push(watch_views.to_path_buf());
    }
    if let Some(cfg) = cfg {
        for resource in &cfg.resources {
            let path = cwd.join(resource);
            if path.exists() && !dirs.iter().any(|dir| dir == &path) {
                dirs.push(path);
            }
        }
    }
    dirs
}

fn watched_description(sources: &[PathBuf], resources: &[PathBuf]) -> String {
    sources
        .iter()
        .chain(resources.iter())
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
