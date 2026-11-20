//! Pure-Rust MCP `2024-11-05` stdio loop for `rsicraft`.
//!
//! Invariants (`RUST_ARTCRAFT_STANDARD.md` Gate 4):
//! - `stdout` carries 100% pure line-delimited JSON-RPC; diagnostics go to `stderr`.
//! - Notifications (requests without `id`) never emit a response.
//! - `handle_line` is a pure function so protocol behavior is unit-tested in memory.
//! - Both `rsi_step` and dotted `rsi.step` tool names are accepted.

use serde_json::{json, Value};
use std::io::{BufRead, Write};

pub const PROTOCOL_VERSION: &str = "2024-11-05";
pub const MAX_LINE_BYTES: usize = 4 * 1024 * 1024;

pub trait ToolHost {
    fn server_name(&self) -> &str;
    fn server_version(&self) -> &str;
    fn tools(&self) -> Vec<Value>;
    fn call(&mut self, name: &str, args: &Value) -> Result<Value, String>;
    fn resources(&self) -> Vec<Value> {
        Vec::new()
    }
    fn read_resource(&self, uri: &str) -> Result<Value, String> {
        Err(format!("unknown resource {uri}"))
    }
}

#[must_use]
pub fn normalize_tool_name(name: &str) -> String {
    name.replace('.', "_")
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}

fn ok_response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

pub fn handle_line<H: ToolHost>(host: &mut H, line: &str) -> Option<Value> {
    let trimmed = line.trim_start_matches('\u{feff}').trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.len() > MAX_LINE_BYTES {
        return Some(error_response(Value::Null, -32600, "request too large"));
    }
    let req: Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(_) => return Some(error_response(Value::Null, -32700, "parse error")),
    };
    if !req.is_object() {
        return Some(error_response(
            Value::Null,
            -32600,
            "request must be a JSON object",
        ));
    }
    let id = match req.get("id") {
        Some(v) if !v.is_null() => v.clone(),
        _ => return None,
    };
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(Value::Null);

    Some(match method {
        "initialize" => ok_response(
            id,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {
                    "tools": { "listChanged": false },
                    "resources": { "listChanged": false }
                },
                "serverInfo": {
                    "name": host.server_name(),
                    "version": host.server_version()
                }
            }),
        ),
        "ping" => ok_response(id, json!({})),
        "tools/list" => ok_response(id, json!({ "tools": host.tools() })),
        "resources/list" => ok_response(id, json!({ "resources": host.resources() })),
        "resources/read" => {
            let Some(uri) = params.get("uri").and_then(Value::as_str) else {
                return Some(error_response(
                    id,
                    -32602,
                    "resources/read requires params.uri",
                ));
            };
            match host.read_resource(uri) {
                Ok(v) => ok_response(id, v),
                Err(e) => error_response(id, -32602, &e),
            }
        }
        "tools/call" => {
            let Some(raw_name) = params.get("name").and_then(Value::as_str) else {
                return Some(error_response(
                    id,
                    -32602,
                    "tools/call requires params.name",
                ));
            };
            let name = normalize_tool_name(raw_name);
            let known = host
                .tools()
                .iter()
                .any(|t| t.get("name").and_then(Value::as_str) == Some(name.as_str()));
            if !known {
                return Some(error_response(id, -32602, &format!("unknown tool {name}")));
            }
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            if !args.is_object() {
                return Some(error_response(id, -32602, "arguments must be an object"));
            }
            let (text, is_error) = match host.call(&name, &args) {
                Ok(v) => (
                    serde_json::to_string_pretty(&v).unwrap_or_else(|e| e.to_string()),
                    false,
                ),
                Err(e) => (json!({ "error": e }).to_string(), true),
            };
            ok_response(
                id,
                json!({
                    "content": [{ "type": "text", "text": text }],
                    "isError": is_error
                }),
            )
        }
        other => error_response(id, -32601, &format!("method not found: {other}")),
    })
}

pub fn run_stdio<H: ToolHost, R: BufRead, W: Write>(
    host: &mut H,
    reader: R,
    mut writer: W,
) -> std::io::Result<()> {
    eprintln!(
        "[{}] MCP {PROTOCOL_VERSION} stdio ready",
        host.server_name()
    );
    for line in reader.lines() {
        let line = line?;
        if let Some(resp) = handle_line(host, &line) {
            let s = serde_json::to_string(&resp).unwrap_or_else(|e| {
                format!(
                    "{{\"jsonrpc\":\"2.0\",\"id\":null,\"error\":{{\"code\":-32603,\"message\":\"{e}\"}}}}"
                )
            });
            writer.write_all(s.as_bytes())?;
            writer.write_all(b"\n")?;
            writer.flush()?;
        }
    }
    Ok(())
}

pub fn serve_stdio<H: ToolHost>(host: &mut H) -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    run_stdio(host, stdin.lock(), stdout.lock())
}
