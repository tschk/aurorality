//! Default `aurorality dev`: no server, no port; the app gets the live views dir
//! and, if it watches its templates (marker file), a template save does not
//! rebuild or relaunch it.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct KillOnDrop(Child);
impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_until(timeout: Duration, mut f: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if f() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    f()
}

fn launches(log: &Path) -> Vec<String> {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .map(String::from)
        .collect()
}

/// Runs `aurorality dev` on a fake project whose "app" logs its environment and,
/// when `announce` is set, creates the live marker like `watchLiveTemplate`.
fn run(announce: bool, observer: bool) -> Vec<String> {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("bin");
    let project = dir.path().join("proj");
    let log = dir.path().join("launches.log");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::create_dir_all(project.join("views")).unwrap();
    let marker_cmd = if announce {
        ": > \"$AURORALITY_LIVE_MARKER\""
    } else {
        ":"
    };
    let app = format!(
        "#!/bin/sh\\necho \"views=$AURORALITY_VIEWS_DIR port=${{AURORALITY_DEV_PORT:-none}} overlay=${{AURORALITY_DEV:-off}}\" >> \"$DEV_LIVE_LOG\"\\n{marker_cmd}\\nexec sleep 30\\n"
    );
    let swift = format!(
        "#!/bin/sh\nmkdir -p .build/debug/LiveFixture_LiveFixture.bundle\n: > .build/debug/LiveFixture_LiveFixture.bundle/main.crepus\nprintf '{app}' > .build/debug/LiveFixture\nchmod +x .build/debug/LiveFixture\nexit 0\n"
    );
    for (name, body) in [
        ("cargo", "#!/bin/sh\nexit 0\n".to_string()),
        ("swift", swift),
    ] {
        let p = bin.join(name);
        std::fs::write(&p, body).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    std::fs::write(
        project.join("Package.swift"),
        "let package = Package(\n    name: \"LiveFixture\",\n)\n",
    )
    .unwrap();
    std::fs::write(
        project.join(".brisk.toml"),
        "[package]\nname = \"FixtureApp\"\n",
    )
    .unwrap();
    let template = project.join("views/main.crepus");
    std::fs::write(&template, "span\n  \"one\"\n").unwrap();

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let mut command = Command::new(env!("CARGO_BIN_EXE_aurorality"));
    command.arg("dev");
    if observer {
        command.args(["--serve", "--port", &port.to_string()]);
    }
    let _dev = KillOnDrop(
        command
            .current_dir(&project)
            .env("CARGO", bin.join("cargo"))
            .env("SWIFT_PATH", bin.join("swift"))
            .env("DEV_LIVE_LOG", &log)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    assert!(wait_until(Duration::from_secs(20), || launches(&log).len() == 1));
    let _observer = if observer {
        Some(
            tungstenite::connect(format!("ws://127.0.0.1:{port}"))
                .unwrap()
                .0,
        )
    } else {
        None
    };
    std::thread::sleep(Duration::from_millis(500));
    std::fs::write(&template, "span\n  \"two\"\n").unwrap();
    // Debounce is 300 ms; give a relaunch ample time to happen if it is going to.
    wait_until(Duration::from_secs(4), || launches(&log).len() > 1);
    assert!(
        project
            .join(".build/FixtureApp.app/Contents/Resources/LiveFixture_LiveFixture.bundle/main.crepus")
            .is_file(),
        "SwiftPM resource bundle missing from the macOS app (Bundle.module would trap)"
    );
    let out = launches(&log);
    // The fake apps (`sleep`) are children of the killed app processes; reap by name is
    // not needed because `dev` kills its child on shutdown.
    out
}

#[test]
fn watching_app_is_not_relaunched_on_template_save_and_no_port_is_used() {
    let launches = run(true, false);
    assert_eq!(launches.len(), 1, "{launches:?}");
    // No server, so the dev overlay (which would connect to one) stays off.
    assert!(
        launches[0].ends_with("/views port=none overlay=off"),
        "{launches:?}"
    );
    assert!(
        launches[0].starts_with("views=/"),
        "views dir must be absolute: {launches:?}"
    );
}

#[test]
fn app_that_does_not_watch_is_rebuilt_on_template_save() {
    let launches = run(false, false);
    assert_eq!(launches.len(), 2, "{launches:?}");
}

#[test]
fn unrelated_websocket_client_does_not_prevent_relaunch() {
    let launches = run(false, true);
    assert_eq!(launches.len(), 2, "{launches:?}");
}
