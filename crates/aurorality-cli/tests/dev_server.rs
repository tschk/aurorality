//! `aurorality dev` must serve the WebSocket protocol `HotReloadClient` expects.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Kills `aurorality dev` even if an assertion panics.
struct KillOnDrop(Child);
impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn dev_serves_hot_reload_over_websocket() {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("bin");
    let project = dir.path().join("proj");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::create_dir_all(project.join("views")).unwrap();
    // Fake toolchain: Rust pre-build succeeds, the Swift build fails, so no app launches.
    for (name, body) in [
        ("cargo", "#!/bin/sh\nexit 0\n"),
        ("swift", "#!/bin/sh\nexit 1\n"),
    ] {
        let p = bin.join(name);
        std::fs::write(&p, body).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    std::fs::write(
        project.join("Package.swift"),
        "let package = Package(\n    name: \"AurorDevWsFixtureQ7\",\n)\n",
    )
    .unwrap();
    std::fs::write(
        project.join("views/main.crepus"),
        "span text-base\n  \"hello ws\"\n",
    )
    .unwrap();

    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let _child = KillOnDrop(
        Command::new(env!("CARGO_BIN_EXE_aurorality"))
            .args(["dev", "--port", &port.to_string()])
            .current_dir(&project)
            .env("CARGO", bin.join("cargo"))
            .env("SWIFT_PATH", bin.join("swift"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );

    let url = format!("ws://127.0.0.1:{port}");
    let deadline = Instant::now() + Duration::from_secs(15);
    let connected = loop {
        match tungstenite::connect(url.as_str()) {
            Ok((ws, _)) => break Some(ws),
            Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(100)),
            Err(_) => break None,
        }
    };
    let result = connected.map(|mut ws| {
        if let tungstenite::stream::MaybeTlsStream::Plain(s) = ws.get_mut() {
            s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        }
        let hello: serde_json::Value =
            serde_json::from_str(ws.read().unwrap().to_text().unwrap()).unwrap();
        let ir: serde_json::Value =
            serde_json::from_str(ws.read().unwrap().to_text().unwrap()).unwrap();
        (hello, ir)
    });
    let (hello, ir) = result.expect("aurorality dev did not accept a WebSocket connection");
    assert_eq!(hello["message"]["kind"], "devHello", "{hello}");
    assert_eq!(ir["message"]["kind"], "fullReload", "{ir}");
    assert!(ir.to_string().contains("hello ws"), "{ir}");
}
