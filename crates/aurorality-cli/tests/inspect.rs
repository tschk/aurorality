use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Command, Output, Stdio};

fn inspect(template: &str, args: &[&str]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aurorality-inspect"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(template.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn outputs_ir_for_a_template() {
    let output = inspect("\"Hello!\"", &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let ir: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(ir["root"][0]["content"], "Hello!");
}

#[test]
fn renders_supplied_context() {
    let output = inspect("\"{name}\"", &["--context", r#"{"name":"Ada"}"#]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let ir: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(ir["root"][0]["content"], "Ada");
}

#[test]
fn reports_context_errors_without_ir() {
    let output = inspect("\"Hello!\"", &["--context", r#"{"user":{"name":"Ada"}}"#]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("invalid context"), "{error}");
    assert!(error.contains("$.user"), "{error}");
}

#[test]
fn reports_invalid_template_without_ir() {
    let output = inspect("<", &[]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("render error"));
}

struct Playground(std::process::Child, String);

impl Playground {
    fn start() -> Self {
        Self::start_in(None, None)
    }

    fn start_in(cwd: Option<&std::path::Path>, root: Option<&std::path::Path>) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_aurorality-inspect"));
        command.args(["--playground", "--port", "0"]);
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }
        if let Some(root) = root {
            command.arg("--include-root").arg(root);
        }
        let mut child = command.stderr(Stdio::piped()).spawn().unwrap();
        let mut line = String::new();
        BufReader::new(child.stderr.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let address = line
            .trim()
            .strip_prefix("Playground listening on http://")
            .unwrap()
            .to_owned();
        Self(child, address)
    }

    fn request(
        &self,
        method: &str,
        path: &str,
        origin: &str,
        content_type: &str,
        body: &str,
    ) -> (u16, String) {
        self.request_with_host(method, path, &self.1, origin, content_type, body)
    }

    fn request_with_host(
        &self,
        method: &str,
        path: &str,
        host: &str,
        origin: &str,
        content_type: &str,
        body: &str,
    ) -> (u16, String) {
        let mut stream = TcpStream::connect(&self.1).unwrap();
        write!(stream, "{method} {path} HTTP/1.1\r\nHost: {host}\r\nOrigin: {origin}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let (headers, body) = response.split_once("\r\n\r\n").unwrap();
        assert!(!headers
            .to_ascii_lowercase()
            .contains("access-control-allow-origin"));
        (
            headers.split_whitespace().nth(1).unwrap().parse().unwrap(),
            body.to_owned(),
        )
    }
}

#[test]
fn playground_confines_includes_to_configured_root() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("project");
    std::fs::create_dir_all(root.join("nested")).unwrap();
    std::fs::write(root.join("nested/inner.crepus"), "\"inside\"").unwrap();
    std::fs::write(root.join("outer.crepus"), "include nested/inner.crepus").unwrap();
    std::fs::write(dir.path().join("secret.crepus"), "\"secret\"").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(dir.path().join("secret.crepus"), root.join("link.crepus")).unwrap();

    for server in [
        Playground::start_in(Some(&root), None),
        Playground::start_in(Some(dir.path()), Some(&root)),
    ] {
        let origin = format!("http://{}", server.1);
        let render = |path: &str| {
            let body = serde_json::json!({"template": format!("include {path}"), "context": "{}"})
                .to_string();
            server.request("POST", "/render", &origin, "application/json", &body)
        };
        let (status, body) = render("outer.crepus");
        assert_eq!(status, 200, "{body}");
        assert!(body.contains("inside"), "{body}");
        let absolute = dir
            .path()
            .join("secret.crepus")
            .to_string_lossy()
            .into_owned();
        let mut paths = vec!["../secret.crepus", absolute.as_str()];
        #[cfg(unix)]
        paths.push("link.crepus");
        for path in paths {
            let (status, body) = render(path);
            assert_eq!(status, 400, "{body}");
            assert!(body.contains(path), "{body}");
        }
        std::fs::write(
            root.join("nested/escape.crepus"),
            "include ../../secret.crepus",
        )
        .unwrap();
        let (status, body) = render("nested/escape.crepus");
        assert_eq!(status, 400, "{body}");
        assert!(body.contains("../../secret.crepus"), "{body}");
    }
}

impl Drop for Playground {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn playground_endpoints_and_validation() {
    let server = Playground::start();
    let origin = format!("http://{}", server.1);
    let (status, html) = server.request("GET", "/", &origin, "text/plain", "");
    assert_eq!(status, 200);
    assert!(!html.is_empty());
    let body =
        serde_json::json!({"template": "\"{name}\"", "context": "{\"name\":\"Ada\"}"}).to_string();
    let (status, response) = server.request("POST", "/render", &origin, "application/json", &body);
    assert_eq!(status, 200);
    let json: serde_json::Value = serde_json::from_str(&response).unwrap();
    assert_eq!(json["ir"]["root"][0]["content"], "Ada");
    let invalid = serde_json::json!({"template": "\"hi\"", "context": "[]"}).to_string();
    let (status, response) =
        server.request("POST", "/render", &origin, "application/json", &invalid);
    assert_eq!(status, 400);
    assert!(serde_json::from_str::<serde_json::Value>(&response).unwrap()["error"].is_string());
    assert_eq!(
        server
            .request(
                "POST",
                "/render",
                "https://evil.example",
                "application/json",
                &body
            )
            .0,
        403
    );
    assert_eq!(
        server
            .request("POST", "/render", &origin, "text/plain", &body)
            .0,
        415
    );
    assert_eq!(
        server
            .request(
                "POST",
                "/render",
                &origin,
                "application/json",
                &"x".repeat(1024 * 1024 + 1)
            )
            .0,
        413
    );
    assert_eq!(
        server
            .request("GET", "/Cargo.toml", &origin, "text/plain", "")
            .0,
        404
    );
}

#[test]
fn playground_rejects_rebinding_and_wrong_port() {
    let server = Playground::start();
    let port = server.1.rsplit_once(':').unwrap().1;
    let body = r#"{"template":"\"hi\"","context":"{}"}"#;
    for host in [
        format!("attacker.example:{port}"),
        "127.0.0.1:1".to_owned(),
        format!("localhost.attacker.example:{port}"),
    ] {
        let origin = format!("http://{host}");
        assert_eq!(
            server
                .request_with_host("GET", "/", &host, &origin, "text/plain", "")
                .0,
            403
        );
        assert_eq!(
            server
                .request_with_host("POST", "/render", &host, &origin, "application/json", body)
                .0,
            403
        );
    }
    let host = format!("localhost:{port}");
    let origin = format!("http://{host}");
    assert_eq!(
        server
            .request_with_host("GET", "/", &host, &origin, "text/plain", "")
            .0,
        200
    );
    assert_eq!(
        server
            .request_with_host("POST", "/render", &host, &origin, "application/json", body)
            .0,
        200
    );
    assert_eq!(
        server
            .request_with_host(
                "POST",
                "/render",
                &host,
                &format!("https://{host}"),
                "application/json",
                body
            )
            .0,
        403
    );
}
