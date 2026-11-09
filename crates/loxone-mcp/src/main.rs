use std::path::PathBuf;

use loxone_mcp::protocol::DEFAULT_GUI_MCP_ADDR;
use loxone_mcp::session::HeadlessSession;
use loxone_mcp::transport::{run_http_server, run_stdio_server};

fn print_help() {
    println!(
        "loxone-mcp {} — Model Context Protocol (MCP) server for Loxone Portable (.Loxone projects)\n\
\n\
USAGE:\n\
    loxone-mcp [OPTIONS]\n\
\n\
OPTIONS:\n\
    --project <PATH>       Load initial .Loxone project file on startup\n\
    --http <HOST:PORT>     Run headless Streamable HTTP JSON-RPC server (e.g. 127.0.0.1:8765)\n\
                           instead of stdio\n\
    --headless             Do not proxy tool calls to a running loxone-gui instance\n\
    --gui-url <URL>        Override live GUI bridge endpoint (default: http://{}/mcp)\n\
    --no-auto-save         Do not automatically write edits back to --project on disk\n\
    -h, --help             Print help information",
        env!("CARGO_PKG_VERSION"),
        DEFAULT_GUI_MCP_ADDR
    );
}

fn main() {
    let mut initial_project: Option<PathBuf> = None;
    let mut http_bind: Option<String> = None;
    let mut headless_only = false;
    let mut auto_save = true;
    let mut gui_url = std::env::var("LOXONE_MCP_GUI_URL")
        .unwrap_or_else(|_| format!("http://{DEFAULT_GUI_MCP_ADDR}/mcp"));

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "--project" | "-p" => {
                if let Some(p) = args.next() {
                    initial_project = Some(PathBuf::from(p));
                }
            }
            "--http" => {
                let addr = args
                    .next()
                    .unwrap_or_else(|| DEFAULT_GUI_MCP_ADDR.to_string());
                http_bind = Some(addr);
            }
            "--headless" => {
                headless_only = true;
            }
            "--no-auto-save" => {
                auto_save = false;
            }
            "--gui-url" => {
                if let Some(u) = args.next() {
                    gui_url = u;
                }
            }
            other if !other.starts_with('-') && initial_project.is_none() => {
                initial_project = Some(PathBuf::from(other));
            }
            _ => {}
        }
    }

    let session = match HeadlessSession::new(initial_project.as_deref(), auto_save) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("loxone-mcp startup error: {e}");
            std::process::exit(1);
        }
    };

    if let Some(bind_addr) = http_bind {
        if let Err(e) = run_http_server(&bind_addr, session) {
            eprintln!("loxone-mcp HTTP error: {e}");
            std::process::exit(1);
        }
    } else if let Err(e) = run_stdio_server(session, headless_only, &gui_url) {
        eprintln!("loxone-mcp stdio error: {e}");
        std::process::exit(1);
    }
}
