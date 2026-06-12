//! `aurorality dev` — watch → rebuild → relaunch loop.
//!
//! Watches `views/` and `Sources/`. On change: swift build, kill old app,
//! launch new app. No WebSocket, no ports — same pattern as `crepus dev`.

use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecursiveMode, Watcher};

use crate::build_swift;

/// Build from scratch, then watch → rebuild → relaunch.
pub fn run(watch_views: PathBuf) {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Pre-build Rust + bindings (only once)
    if let Err(e) = crate::pre_build() {
        eprintln!("pre-build failed: {e}");
        return;
    }

    let child: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));
    let shutdown = Arc::new(AtomicBool::new(false));
    let child_for_signal = Arc::clone(&child);
    let shutdown_for_signal = Arc::clone(&shutdown);
    let cwd_for_signal = cwd.clone();
    if let Err(e) = ctrlc::set_handler(move || {
        shutdown_for_signal.store(true, Ordering::Relaxed);
        stop_running_app(&cwd_for_signal, &child_for_signal, false);
    }) {
        eprintln!("could not install Ctrl-C handler: {e}");
    }

    // Initial build + launch
    let cfg = build_swift::read_config(&cwd).ok();
    let watched_sources = watched_source_dirs(&cwd, cfg.as_ref());
    let watched_resources = watched_resource_dirs(&cwd, &watch_views, cfg.as_ref());
    println!(
        "{} watching {} — edit to rebuild",
        console::style("aurorality dev").cyan().bold(),
        watched_description(&watched_sources, &watched_resources),
    );
    {
        let mut child = child.lock().unwrap();
        *child = build_and_launch_app(&cwd);
    }

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
        Err(e) => {
            eprintln!("could not create file watcher: {e}");
            return;
        }
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

    loop {
        if shutdown.load(Ordering::Relaxed) {
            break;
        }

        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(_) => {
                // Debounce: drain for 300ms
                let t = Instant::now();
                while t.elapsed() < Duration::from_millis(300) {
                    while rx.try_recv().is_ok() {}
                    std::thread::sleep(Duration::from_millis(30));
                }

                stop_running_app(&cwd, &child, true);

                // Rebuild + relaunch
                let mut new = child.lock().unwrap();
                *new = build_and_launch_app(&cwd);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    stop_running_app(&cwd, &child, false);
}

fn build_and_launch_app(project_root: &PathBuf) -> Option<Child> {
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

            // Kill old instances before launching new one
            if let Ok(cfg) = build_swift::read_config(project_root) {
                let _ = Command::new("pkill").args(["-f", &cfg.name]).status();
            }

            match build_swift::build_and_launch_spawn(project_root) {
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

fn stop_running_app(project_root: &PathBuf, child: &Arc<Mutex<Option<Child>>>, restarting: bool) {
    if let Ok(mut old) = child.lock() {
        if let Some(mut c) = old.take() {
            if restarting {
                println!("  restarting…");
            }
            let _ = c.kill();
            let _ = c.wait();
        }
    }

    if let Ok(cfg) = build_swift::read_config(project_root) {
        let _ = Command::new("pkill").args(["-f", &cfg.name]).status();
    }
}

fn watched_source_dirs(cwd: &PathBuf, cfg: Option<&build_swift::ProjectConfig>) -> Vec<PathBuf> {
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
    cwd: &PathBuf,
    watch_views: &PathBuf,
    cfg: Option<&build_swift::ProjectConfig>,
) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if watch_views.exists() {
        dirs.push(watch_views.clone());
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
