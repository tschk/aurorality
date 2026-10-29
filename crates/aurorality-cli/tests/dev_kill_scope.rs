//! `aurorality dev` must only stop processes it started, never `pkill -f <name>`.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const NAME: &str = "AurorKillScopeFixtureZ3";

#[test]
fn dev_does_not_kill_unrelated_processes_sharing_the_project_name() {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("bin");
    let project = dir.path().join("proj");
    let marker = dir.path().join("launched");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::create_dir_all(project.join("views")).unwrap();
    // Fake toolchain: pre-build succeeds; `swift build` emits an "app" that records its PID.
    let swift = format!(
        "#!/bin/sh\nmkdir -p .build/debug\nprintf '#!/bin/sh\\necho $$ > \"$KILL_SCOPE_MARKER\"\\nexec sleep 30\\n' > .build/debug/{NAME}\nchmod +x .build/debug/{NAME}\nexit 0\n"
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
        format!("let package = Package(\n    name: \"{NAME}\",\n)\n"),
    )
    .unwrap();

    // An unrelated process whose command line contains the project name.
    let mut bystander = Command::new("sh")
        .args(["-c", "sleep 30; :", NAME])
        .spawn()
        .unwrap();

    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut dev = Command::new(env!("CARGO_BIN_EXE_aurorality"))
        .args(["dev", "--port", &port.to_string()])
        .current_dir(&project)
        .env("CARGO", bin.join("cargo"))
        .env("SWIFT_PATH", bin.join("swift"))
        .env("KILL_SCOPE_MARKER", &marker)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(20);
    while !marker.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(300));
    let launched = marker.exists();
    let bystander_alive = bystander.try_wait().unwrap().is_none();

    let _ = dev.kill();
    let _ = dev.wait();
    if let Ok(pid) = std::fs::read_to_string(&marker) {
        let _ = Command::new("kill").arg(pid.trim()).status();
    }
    let _ = bystander.kill();
    let _ = bystander.wait();

    assert!(launched, "dev never launched the fake app");
    assert!(
        bystander_alive,
        "dev killed an unrelated process named like the project"
    );
}
