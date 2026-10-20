//! Built-in MCP server (Streamable HTTP, JSON-RPC 2.0) so AI agents can drive Flightdeck:
//! inspect and act on the engine, and operate the UI itself. Tools run on the UI thread via a
//! queue drained every frame; the HTTP thread blocks until the result arrives.

pub mod tools;

use serde_json::{json, Value};
use std::collections::VecDeque;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const PROTOCOL_VERSION: &str = "2025-06-18";

pub struct McpCall {
    pub tool: String,
    pub args: Value,
    pub reply: Reply,
}

/// Outcome of a tool call: a JSON value or an error message for the agent.
pub type ToolResult = Result<Value, String>;

/// Reply handle; cloneable so async completions can answer later.
#[derive(Clone)]
pub struct Reply(SyncSender<ToolResult>);

impl Reply {
    pub fn new(tx: SyncSender<ToolResult>) -> Self {
        Self(tx)
    }
    pub fn ok(&self, v: Value) {
        let _ = self.0.send(Ok(v));
    }
    pub fn err(&self, e: impl Into<String>) {
        let _ = self.0.send(Err(e.into()));
    }
    pub fn send(&self, r: ToolResult) {
        let _ = self.0.send(r);
    }
}

pub type Queue = Arc<Mutex<VecDeque<McpCall>>>;

#[derive(Default, Clone, Debug)]
pub struct McpStats {
    pub requests: u64,
    pub tool_calls: u64,
    pub errors: u64,
    pub last_tool: Option<String>,
    pub last_at: Option<Instant>,
    pub clients: u64,
}

pub struct McpServer {
    pub port: u16,
    server: Arc<tiny_http::Server>,
    thread: Option<std::thread::JoinHandle<()>>,
    pub stats: Arc<Mutex<McpStats>>,
}

impl McpServer {
    pub fn start(port: u16, token: Option<String>, queue: Queue, ctx: egui::Context) -> std::io::Result<Self> {
        let server = tiny_http::Server::http(("127.0.0.1", port)).map_err(|e| std::io::Error::other(e.to_string()))?;
        let server = Arc::new(server);
        let stats = Arc::new(Mutex::new(McpStats::default()));
        let s2 = server.clone();
        let st2 = stats.clone();
        let thread = std::thread::Builder::new().name("flightdeck-mcp".into()).spawn(move || {
            for req in s2.incoming_requests() {
                let q = queue.clone();
                let c = ctx.clone();
                let t = token.clone();
                let st = st2.clone();
                // one thread per request so a slow tool does not block others
                std::thread::spawn(move || handle(req, q, c, t, st));
            }
        })?;
        Ok(Self { port, server, thread: Some(thread), stats })
    }

    pub fn stop(mut self) {
        self.server.unblock();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}/mcp", self.port)
    }
}

fn cors(resp: tiny_http::Response<std::io::Cursor<Vec<u8>>>) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    resp.with_header(tiny_http::Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap())
        .with_header(tiny_http::Header::from_bytes("Access-Control-Allow-Headers", "Content-Type, Authorization, Mcp-Session-Id, Mcp-Protocol-Version").unwrap())
        .with_header(tiny_http::Header::from_bytes("Access-Control-Allow-Methods", "GET, POST, OPTIONS, DELETE").unwrap())
}

fn json_response(status: u16, body: &Value) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    let data = serde_json::to_vec(body).unwrap_or_default();
    cors(tiny_http::Response::from_data(data).with_status_code(status).with_header(tiny_http::Header::from_bytes("Content-Type", "application/json").unwrap()))
}

fn handle(mut req: tiny_http::Request, queue: Queue, ctx: egui::Context, token: Option<String>, stats: Arc<Mutex<McpStats>>) {
    stats.lock().unwrap().requests += 1;
    let method = req.method().to_string();
    let url = req.url().split('?').next().unwrap_or("").to_string();
    if method == "OPTIONS" {
        let _ = req.respond(cors(tiny_http::Response::from_data(Vec::new()).with_status_code(204)));
        return;
    }
    if url == "/health" || url == "/" {
        let _ = req.respond(json_response(200, &json!({"ok": true, "server": "flightdeck", "mcp": "/mcp", "protocol": PROTOCOL_VERSION})));
        return;
    }
    if url != "/mcp" {
        let _ = req.respond(json_response(404, &json!({"error": "not found"})));
        return;
    }
    if let Some(t) = &token {
        let auth = req.headers().iter().find(|h| h.field.equiv("Authorization")).map(|h| h.value.as_str().to_string()).unwrap_or_default();
        if auth.trim() != format!("Bearer {t}") {
            let _ = req.respond(json_response(401, &json!({"error": "unauthorized: send Authorization: Bearer <token>"})));
            return;
        }
    }
    if method == "GET" {
        // No server-initiated stream; clients fall back to plain request/response.
        let _ = req.respond(json_response(405, &json!({"error": "SSE stream not offered; POST JSON-RPC to /mcp"})));
        return;
    }
    if method == "DELETE" {
        let _ = req.respond(cors(tiny_http::Response::from_data(Vec::new()).with_status_code(204)));
        return;
    }
    let mut body = String::new();
    if req.as_reader().read_to_string(&mut body).is_err() {
        let _ = req.respond(json_response(400, &rpc_error(Value::Null, -32700, "cannot read body")));
        return;
    }
    let parsed: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            let _ = req.respond(json_response(400, &rpc_error(Value::Null, -32700, &format!("parse error: {e}"))));
            return;
        }
    };
    let batch = parsed.is_array();
    let msgs: Vec<Value> = if batch { parsed.as_array().cloned().unwrap_or_default() } else { vec![parsed] };
    let mut responses = vec![];
    for m in msgs {
        if let Some(r) = handle_message(&m, &queue, &ctx, &stats) {
            responses.push(r);
        }
    }
    if responses.is_empty() {
        let _ = req.respond(cors(tiny_http::Response::from_data(Vec::new()).with_status_code(202)));
    } else if batch {
        let _ = req.respond(json_response(200, &Value::Array(responses)));
    } else {
        let _ = req.respond(json_response(200, &responses.remove(0)));
    }
}

fn rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn rpc_ok(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

pub const INSTRUCTIONS: &str = "Flightdeck is a desktop cockpit for a Camunda 7 engine. Tools starting with `ui_` drive the running UI (navigate, select elements, zoom, dock tabs, screenshots); the others read or change the engine (definitions, instances, variables, incidents, jobs, tasks, modification, batches). Start with `engine_status` and `ui_state`. Destructive tools require `confirm: true`. Instance search uses the query language: def:key bk:ABC* var.amount>1000 act:Task_x exec:Task_y state:finished started>-7d.";

fn handle_message(m: &Value, queue: &Queue, ctx: &egui::Context, stats: &Arc<Mutex<McpStats>>) -> Option<Value> {
    let id = m.get("id").cloned().unwrap_or(Value::Null);
    let method = m.get("method").and_then(|v| v.as_str()).unwrap_or("");
    let is_notification = m.get("id").is_none();
    match method {
        "initialize" => {
            stats.lock().unwrap().clients += 1;
            let requested = m.pointer("/params/protocolVersion").and_then(|v| v.as_str()).unwrap_or(PROTOCOL_VERSION);
            let version = if ["2024-11-05", "2025-03-26", "2025-06-18"].contains(&requested) { requested } else { PROTOCOL_VERSION };
            Some(rpc_ok(id, json!({
                "protocolVersion": version,
                "capabilities": {"tools": {"listChanged": false}, "resources": {}, "prompts": {}},
                "serverInfo": {"name": "flightdeck", "title": "Flightdeck (Camunda 7 cockpit)", "version": env!("CARGO_PKG_VERSION")},
                "instructions": INSTRUCTIONS,
            })))
        }
        "ping" => Some(rpc_ok(id, json!({}))),
        "tools/list" => Some(rpc_ok(id, json!({"tools": tools::specs()}))),
        "resources/list" => Some(rpc_ok(id, json!({"resources": []}))),
        "resources/templates/list" => Some(rpc_ok(id, json!({"resourceTemplates": []}))),
        "prompts/list" => Some(rpc_ok(id, json!({"prompts": tools::prompts()}))),
        "prompts/get" => {
            let name = m.pointer("/params/name").and_then(|v| v.as_str()).unwrap_or("");
            Some(match tools::prompt(name) {
                Some(p) => rpc_ok(id, p),
                None => rpc_error(id, -32602, "unknown prompt"),
            })
        }
        "tools/call" => {
            let name = m.pointer("/params/name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let args = m.pointer("/params/arguments").cloned().unwrap_or(json!({}));
            {
                let mut s = stats.lock().unwrap();
                s.tool_calls += 1;
                s.last_tool = Some(name.clone());
                s.last_at = Some(Instant::now());
            }
            if !tools::exists(&name) {
                stats.lock().unwrap().errors += 1;
                return Some(rpc_ok(id, tool_result(Err(format!("unknown tool {name}")))));
            }
            let (tx, rx) = sync_channel::<ToolResult>(1);
            queue.lock().unwrap().push_back(McpCall { tool: name.clone(), args, reply: Reply(tx) });
            ctx.request_repaint();
            let r = match rx.recv_timeout(Duration::from_secs(180)) {
                Ok(r) => r,
                Err(_) => Err("timed out waiting for the UI thread (is the window alive?)".to_string()),
            };
            if r.is_err() {
                stats.lock().unwrap().errors += 1;
            }
            ctx.request_repaint();
            Some(rpc_ok(id, tool_result(r)))
        }
        _ if method.starts_with("notifications/") => None,
        _ => {
            if is_notification {
                None
            } else {
                Some(rpc_error(id, -32601, &format!("method not found: {method}")))
            }
        }
    }
}

/// Convert a tool outcome into MCP `CallToolResult`. A `{"_png_base64": "...", ...}` value becomes image content.
fn tool_result(r: ToolResult) -> Value {
    match r {
        Ok(v) => {
            if let Some(png) = v.get("_png_base64").and_then(|p| p.as_str()) {
                let mut meta = v.clone();
                meta.as_object_mut().map(|o| o.remove("_png_base64"));
                json!({"content": [
                    {"type": "image", "data": png, "mimeType": "image/png"},
                    {"type": "text", "text": serde_json::to_string(&meta).unwrap_or_default()}
                ], "isError": false})
            } else {
                let text = match &v {
                    Value::String(s) => s.clone(),
                    other => serde_json::to_string_pretty(other).unwrap_or_default(),
                };
                let mut out = json!({"content": [{"type": "text", "text": text}], "isError": false});
                if v.is_object() {
                    out["structuredContent"] = v;
                }
                out
            }
        }
        Err(e) => json!({"content": [{"type": "text", "text": e}], "isError": true}),
    }
}

/// `cockpit --mcp-stdio URL`: forward JSON-RPC lines from stdin to the HTTP endpoint and print
/// responses to stdout, for MCP clients that only support stdio transports.
pub fn stdio_bridge(url: &str) {
    use std::io::{BufRead, Write};
    let agent = reqwest::blocking::Client::builder().timeout(Duration::from_secs(200)).build().expect("http client");
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        match agent.post(url).header("Content-Type", "application/json").body(line.clone()).send() {
            Ok(resp) => {
                let status = resp.status();
                let text = resp.text().unwrap_or_default();
                if status.as_u16() == 202 || text.trim().is_empty() {
                    continue; // notification
                }
                let _ = writeln!(out, "{}", text.trim());
                let _ = out.flush();
            }
            Err(e) => {
                let id = serde_json::from_str::<Value>(&line).ok().and_then(|v| v.get("id").cloned()).unwrap_or(Value::Null);
                let _ = writeln!(out, "{}", rpc_error(id, -32000, &format!("Flightdeck MCP endpoint unreachable at {url}: {e}. Start the app and enable MCP in Settings.")));
                let _ = out.flush();
            }
        }
    }
}
