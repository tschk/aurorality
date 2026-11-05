use std::io::Write;
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
