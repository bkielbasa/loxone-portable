use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use serde_json::json;

use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::session::{handle_jsonrpc_request, HeadlessSession, McpSession};

fn parse_http_request(stream: &mut TcpStream) -> Result<(String, String, String), String> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    if reader
        .read_line(&mut request_line)
        .map_err(|e| e.to_string())?
        == 0
    {
        return Err("Empty HTTP request".to_string());
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_uppercase();
    let path = parts.next().unwrap_or("/").to_string();

    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if n == 0 || line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                content_length = v.trim().parse::<usize>().unwrap_or(0);
            }
        }
    }

    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body).map_err(|e| e.to_string())?;
    }
    let body_str = String::from_utf8_lossy(&body).into_owned();
    Ok((method, path, body_str))
}

fn write_http_json_response(stream: &mut TcpStream, status: &str, body: &str) {
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(resp.as_bytes());
    let _ = stream.flush();
}

pub fn forward_to_gui_bridge(bridge_url: &str, req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let payload = serde_json::to_string(req).ok()?;
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_millis(180))
        .timeout_read(Duration::from_secs(5))
        .timeout_write(Duration::from_secs(2))
        .build();
    let resp = agent
        .post(bridge_url)
        .set("Content-Type", "application/json")
        .send_string(&payload)
        .ok()?;
    let text = resp.into_string().ok()?;
    serde_json::from_str::<JsonRpcResponse>(&text).ok()
}

pub fn run_stdio_server(
    mut session: HeadlessSession,
    headless_only: bool,
    gui_bridge_url: &str,
) -> Result<(), String> {
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut stdout = std::io::stdout();

    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let (raw_json, use_content_length) =
            if trimmed.to_ascii_lowercase().starts_with("content-length:") {
                let len = trimmed
                    .split_once(':')
                    .and_then(|(_, v)| v.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                loop {
                    let mut hdr = String::new();
                    let hn = reader.read_line(&mut hdr).map_err(|e| e.to_string())?;
                    if hn == 0 || hdr == "\r\n" || hdr == "\n" {
                        break;
                    }
                }
                let mut buf = vec![0u8; len];
                reader.read_exact(&mut buf).map_err(|e| e.to_string())?;
                (String::from_utf8_lossy(&buf).into_owned(), true)
            } else {
                (trimmed.to_string(), false)
            };

        let req: JsonRpcRequest = match serde_json::from_str(&raw_json) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = JsonRpcResponse::err(None, -32700, format!("Parse error: {e}"));
                write_stdio_message(&mut stdout, &err_resp, use_content_length)?;
                continue;
            }
        };

        let resp_opt = if !headless_only
            && matches!(req.method.as_str(), "tools/call" | "resources/read")
        {
            if let Some(gui_resp) = forward_to_gui_bridge(gui_bridge_url, &req) {
                Some(gui_resp)
            } else {
                handle_jsonrpc_request(&mut session, &req)
            }
        } else {
            handle_jsonrpc_request(&mut session, &req)
        };

        if let Some(resp) = resp_opt {
            write_stdio_message(&mut stdout, &resp, use_content_length)?;
        }
    }
    Ok(())
}

fn write_stdio_message(
    stdout: &mut std::io::Stdout,
    resp: &JsonRpcResponse,
    use_content_length: bool,
) -> Result<(), String> {
    let serialized = serde_json::to_string(resp).map_err(|e| e.to_string())?;
    if use_content_length {
        write!(
            stdout,
            "Content-Length: {}\r\n\r\n{}",
            serialized.len(),
            serialized
        )
        .map_err(|e| e.to_string())?;
    } else {
        writeln!(stdout, "{serialized}").map_err(|e| e.to_string())?;
    }
    stdout.flush().map_err(|e| e.to_string())
}

pub fn run_http_server(bind_addr: &str, mut session: HeadlessSession) -> Result<(), String> {
    let listener = TcpListener::bind(bind_addr)
        .map_err(|e| format!("Failed to bind MCP HTTP server on {bind_addr}: {e}"))?;
    for incoming in listener.incoming() {
        let Ok(mut stream) = incoming else {
            continue;
        };
        let Ok((method, path, body)) = parse_http_request(&mut stream) else {
            continue;
        };
        if method == "OPTIONS" {
            write_http_json_response(&mut stream, "200 OK", "{}");
            continue;
        }
        if method == "GET" && (path == "/health" || path == "/status") {
            let status_json = serde_json::to_string(&json!({
                "status": "ok",
                "mode": session.mode_name(),
                "loaded_path": session.loaded_path().map(|p| p.display().to_string())
            }))
            .unwrap_or_default();
            write_http_json_response(&mut stream, "200 OK", &status_json);
            continue;
        }
        let req: JsonRpcRequest = match serde_json::from_str(&body) {
            Ok(r) => r,
            Err(e) => {
                let err = JsonRpcResponse::err(None, -32700, format!("Invalid JSON-RPC: {e}"));
                let s = serde_json::to_string(&err).unwrap_or_default();
                write_http_json_response(&mut stream, "400 Bad Request", &s);
                continue;
            }
        };
        if let Some(resp) = handle_jsonrpc_request(&mut session, &req) {
            let s = serde_json::to_string(&resp).unwrap_or_default();
            write_http_json_response(&mut stream, "200 OK", &s);
        } else {
            write_http_json_response(&mut stream, "202 Accepted", "{}");
        }
    }
    Ok(())
}

pub struct GuiMcpRequest {
    pub request: JsonRpcRequest,
    pub reply_tx: Sender<Option<JsonRpcResponse>>,
}

pub struct GuiMcpBridge {
    pub bind_addr: String,
    rx: Receiver<GuiMcpRequest>,
}

impl GuiMcpBridge {
    pub fn start<F>(bind_addr: &str, wake_fn: F) -> Option<Self>
    where
        F: Fn() + Send + Sync + 'static,
    {
        let listener = TcpListener::bind(bind_addr).ok()?;
        let actual_addr = listener
            .local_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| bind_addr.to_string());
        let (tx, rx) = mpsc::channel::<GuiMcpRequest>();

        std::thread::Builder::new()
            .name("loxone-gui-mcp-bridge".to_string())
            .spawn(move || {
                for incoming in listener.incoming() {
                    let Ok(mut stream) = incoming else {
                        continue;
                    };
                    let Ok((method, path, body)) = parse_http_request(&mut stream) else {
                        continue;
                    };
                    if method == "OPTIONS" {
                        write_http_json_response(&mut stream, "200 OK", "{}");
                        continue;
                    }
                    if method == "GET" && (path == "/health" || path == "/status") {
                        write_http_json_response(
                            &mut stream,
                            "200 OK",
                            r#"{"status":"ok","mode":"gui_live"}"#,
                        );
                        continue;
                    }
                    let req: JsonRpcRequest = match serde_json::from_str(&body) {
                        Ok(r) => r,
                        Err(e) => {
                            let err = JsonRpcResponse::err(
                                None,
                                -32700,
                                format!("Invalid JSON-RPC: {e}"),
                            );
                            let s = serde_json::to_string(&err).unwrap_or_default();
                            write_http_json_response(&mut stream, "400 Bad Request", &s);
                            continue;
                        }
                    };
                    let (reply_tx, reply_rx) = mpsc::channel();
                    if tx
                        .send(GuiMcpRequest {
                            request: req,
                            reply_tx,
                        })
                        .is_err()
                    {
                        break;
                    }
                    wake_fn();
                    match reply_rx.recv_timeout(Duration::from_secs(5)) {
                        Ok(Some(resp)) => {
                            let s = serde_json::to_string(&resp).unwrap_or_default();
                            write_http_json_response(&mut stream, "200 OK", &s);
                        }
                        Ok(None) => {
                            write_http_json_response(&mut stream, "202 Accepted", "{}");
                        }
                        Err(_) => {
                            let err = JsonRpcResponse::err(
                                None,
                                -32000,
                                "Timed out waiting for GUI frame to process MCP request",
                            );
                            let s = serde_json::to_string(&err).unwrap_or_default();
                            write_http_json_response(&mut stream, "504 Gateway Timeout", &s);
                        }
                    }
                }
            })
            .ok()?;

        Some(Self {
            bind_addr: actual_addr,
            rx,
        })
    }

    pub fn poll(&self, session: &mut dyn McpSession) -> usize {
        let mut processed = 0usize;
        while let Ok(msg) = self.rx.try_recv() {
            let resp = handle_jsonrpc_request(session, &msg.request);
            let _ = msg.reply_tx.send(resp);
            processed += 1;
        }
        processed
    }
}
