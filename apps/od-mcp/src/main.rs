//! `opendjdecks-mcp`: a Model Context Protocol server (revision 2025-06-18)
//! that gives an LLM client access to the OpenDJDecks library for
//! preparing sets with the DJ. Transport: stdio (default) or local HTTP.
//!
//! It opens the same SQLite library as the app (WAL, busy timeout, short
//! write transactions), so both can run at the same time; every write is one
//! undo step in the app's edit history.

mod lighting;
mod server;
mod tools;
mod view;

use anyhow::{Context, bail};
use clap::Parser;
use od_library::Library;
use rmcp::ServiceExt;
use server::OpenDJDecksServer;
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "opendjdecks-mcp",
    version,
    about = "MCP server for preparing sets with an LLM client"
)]
struct Cli {
    /// Library database (default: the app's library: `library` in config.toml, else the platform default).
    #[arg(long)]
    library: Option<PathBuf>,
    /// Serve Streamable HTTP on this loopback address (e.g. 127.0.0.1:8765) at /mcp instead of stdio.
    #[arg(long, value_name = "ADDR")]
    http: Option<SocketAddr>,
}

/// The library the app opens: `library` in `config.toml` next to it, else
/// the default location.
fn library_path(flag: Option<PathBuf>) -> PathBuf {
    #[derive(serde::Deserialize, Default)]
    struct GlobalConfig {
        library: Option<PathBuf>,
    }
    flag.or_else(|| {
        let text = std::fs::read_to_string(od_library::data_dir().join("config.toml")).ok()?;
        toml::from_str::<GlobalConfig>(&text)
            .ok()?
            .library
    })
    .unwrap_or_else(Library::default_path)
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    // stdout carries the protocol: log to stderr only.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .with_env_filter(
            od_core::env::var("MCP_LOG")
                .and_then(|v| tracing_subscriber::EnvFilter::try_new(v).ok())
                .unwrap_or_else(|| tracing_subscriber::EnvFilter::new("warn,rmcp=error")),
        )
        .init();
    let path = library_path(cli.library);
    let lib =
        Library::open(&path).with_context(|| format!("opening library {}", path.display()))?;
    tracing::info!("library {}", path.display());
    let server = OpenDJDecksServer::new(lib);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(async move {
        match cli.http {
            None => {
                let running = server.serve(rmcp::transport::stdio()).await?;
                running.waiting().await?;
            }
            Some(addr) => serve_http(server, addr).await?,
        }
        anyhow::Ok(())
    })
}

async fn serve_http(server: OpenDJDecksServer, addr: SocketAddr) -> anyhow::Result<()> {
    use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
    use rmcp::transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService,
    };
    if !addr.ip().is_loopback() {
        bail!("--http only listens on loopback addresses (127.0.0.1 or [::1]), not {addr}");
    }
    // Loopback `Host` values only (the default) and loopback browser origins:
    // a web page cannot reach the library through DNS rebinding.
    let mut config = StreamableHttpServerConfig::default();
    config.allowed_origins = vec!["http://localhost:*".into(), "http://127.0.0.1:*".into()];
    let sessions = std::sync::Arc::new(LocalSessionManager::default());
    let service = StreamableHttpService::new(move || Ok(server.clone()), sessions, config);
    let router = axum::Router::new().nest_service("/mcp", service);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    eprintln!("opendjdecks-mcp: listening on http://{addr}/mcp");
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
