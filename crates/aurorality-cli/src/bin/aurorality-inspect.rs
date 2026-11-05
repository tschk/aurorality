//! Inspect rendered IR without a SwiftUI runtime.

use std::io::{self, Read};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use serde::Deserialize;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

#[derive(Parser)]
#[command(about = "Render a template from stdin to JSON IR (not a native preview)")]
struct Args {
    /// JSON object containing template variables.
    #[arg(long, default_value = "{}")]
    context: String,
    /// Serve the browser playground on loopback.
    #[arg(long)]
    playground: bool,
    /// Loopback port for the playground (0 selects an available port).
    #[arg(long, default_value_t = 3000, requires = "playground")]
    port: u16,
    /// Directory containing files the playground may include (defaults to the working directory).
    #[arg(long, default_value = ".", requires = "playground")]
    include_root: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.playground {
        return playground(args.port, &args.include_root);
    }
    let mut template = String::new();
    io::stdin()
        .read_to_string(&mut template)
        .context("reading template from stdin")?;
    let ir = aurorality_core::render::render(&template, &args.context)?;
    let value: serde_json::Value = serde_json::from_str(&ir)?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

const MAX_BODY: u64 = 1024 * 1024;

#[derive(Deserialize)]
struct RenderRequest {
    template: String,
    context: String,
}

fn playground(port: u16, include_root: &Path) -> Result<()> {
    let include_root = include_root
        .canonicalize()
        .context("resolving include root")?;
    anyhow::ensure!(
        include_root.is_dir(),
        "include root is not a directory: {}",
        include_root.display()
    );
    let server = Server::http((std::net::Ipv4Addr::LOCALHOST, port))
        .map_err(|e| anyhow::anyhow!("binding playground: {e}"))?;
    let port = server.server_addr().to_ip().expect("TCP listener").port();
    eprintln!("Playground listening on http://{}", server.server_addr());
    for request in server.incoming_requests() {
        handle(request, port, &include_root);
    }
    Ok(())
}

fn header<'a>(request: &'a Request, name: &'static str) -> Option<&'a str> {
    let mut values = request.headers().iter().filter(|h| h.field.equiv(name));
    let value = values.next()?.value.as_str();
    if values.next().is_some() {
        None
    } else {
        Some(value)
    }
}

fn same_origin(request: &Request) -> bool {
    let Some((origin, host)) = header(request, "Origin").zip(header(request, "Host")) else {
        return false;
    };
    let Ok(origin_url) = url::Url::parse(origin) else {
        return false;
    };
    if origin_url.scheme() != "http" || origin_url.origin().ascii_serialization() != origin {
        return false;
    }
    let Ok(host_url) = url::Url::parse(&format!("{}://{host}/", origin_url.scheme())) else {
        return false;
    };
    host_url.origin() == origin_url.origin()
        && host_url.username().is_empty()
        && host_url.password().is_none()
        && host_url.path() == "/"
        && host_url.query().is_none()
        && host_url.fragment().is_none()
}

fn reply(request: Request, status: u16, content_type: &str, body: String) {
    let response = Response::from_string(body)
        .with_status_code(StatusCode(status))
        .with_header(Header::from_bytes("Content-Type", content_type).unwrap())
        .with_header(Header::from_bytes("X-Content-Type-Options", "nosniff").unwrap());
    let _ = request.respond(response);
}

fn error(request: Request, status: u16, message: &str) {
    reply(
        request,
        status,
        "application/json",
        serde_json::json!({"error": message}).to_string(),
    );
}

fn handle(mut request: Request, port: u16, include_root: &Path) {
    let valid_host = header(&request, "Host").is_some_and(|host| {
        ["127.0.0.1", "localhost"].iter().any(|name| {
            host.eq_ignore_ascii_case(&format!("{name}:{port}"))
                || (port == 80 && host.eq_ignore_ascii_case(name))
        })
    });
    if !valid_host {
        error(request, 403, "invalid host");
        return;
    }
    if request.method() == &Method::Get && request.url() == "/" {
        reply(
            request,
            200,
            "text/html; charset=utf-8",
            include_str!("inspect-playground.html").to_owned(),
        );
        return;
    }
    if request.method() != &Method::Post || request.url() != "/render" {
        error(request, 404, "not found");
        return;
    }
    if !same_origin(&request) {
        error(request, 403, "origin does not match host");
        return;
    }
    let json_type = header(&request, "Content-Type")
        .and_then(|v| v.split(';').next())
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("application/json"));
    if !json_type {
        error(request, 415, "content type must be application/json");
        return;
    }
    if request
        .body_length()
        .is_some_and(|len| len as u64 > MAX_BODY)
    {
        error(request, 413, "request body too large");
        return;
    }
    let mut body = Vec::new();
    if request
        .as_reader()
        .take(MAX_BODY + 1)
        .read_to_end(&mut body)
        .is_err()
    {
        error(request, 400, "could not read request body");
        return;
    }
    if body.len() as u64 > MAX_BODY {
        error(request, 413, "request body too large");
        return;
    }
    let Ok(input) = serde_json::from_slice::<RenderRequest>(&body) else {
        error(request, 400, "invalid request JSON");
        return;
    };
    match aurorality_core::render::render_with_include_root(
        &input.template,
        &input.context,
        include_root,
    ) {
        Ok(ir) => match serde_json::from_str::<serde_json::Value>(&ir) {
            Ok(ir) => reply(
                request,
                200,
                "application/json",
                serde_json::json!({"ir": ir}).to_string(),
            ),
            Err(_) => error(request, 500, "invalid renderer output"),
        },
        Err(e) => error(request, 400, &e.to_string()),
    }
}
