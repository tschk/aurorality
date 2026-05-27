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

    // Initial build + launch
    println!(
        "{} watching {} and Sources/ — edit to rebuild",
        console::style("aurorality dev").cyan().bold(),
        watch_views.display(),
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

    // Watch views/
    if watch_views.exists() {
        watcher.watch(&watch_views, RecursiveMode::Recursive).ok();
    }
    // Watch Sources/
    let sources = cwd.join("Sources");
    if sources.exists() {
        watcher.watch(&sources, RecursiveMode::Recursive).ok();
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

                // Kill old app
                {
                    let mut old = child.lock().unwrap();
                    if let Some(mut c) = old.take() {
                        println!("  restarting…");
                        let _ = c.kill();
                        let _ = c.wait();
                    }
                }

                // Rebuild + relaunch
                let mut new = child.lock().unwrap();
                *new = build_and_launch_app(&cwd);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
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
                let _ = Command::new("pkill")
                    .args(["-f", &cfg.name])
                    .status();
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
