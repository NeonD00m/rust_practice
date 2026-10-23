mod detector;
mod server;
mod transport;

use detector::SuiteDetector;
use rmcp::ServiceExt;
use server::McpServer;
use std::env;
use tracing_subscriber::EnvFilter;
use transport::create_stdio_transport;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();

    // Standalone direct terminal probe mode
    if args.iter().any(|arg| arg == "--detect" || arg == "--status" || arg == "-d") {
        let detector = SuiteDetector::new();
        let status = detector.detect_suite();
        println!("=== Bake Productivity Suite Detection ===");
        println!(
            "Installed tools: {} / {}\n",
            status.installed_count,
            detector::SUITE_TOOLS.len()
        );
        for &name in detector::SUITE_TOOLS {
            if let Some(tool) = status.tools.get(name) {
                if tool.installed {
                    println!(
                        "  [✓] {:12} version: {:12} path: {}",
                        name,
                        tool.version.as_deref().unwrap_or("unknown"),
                        tool.path.as_deref().unwrap_or("unknown")
                    );
                } else {
                    println!("  [✗] {:12} (not installed / not found in PATH)", name);
                }
            }
        }
        return Ok(());
    }

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("bake-mcp — Model Context Protocol (MCP) server for the Bake productivity suite\n");
        println!("USAGE:");
        println!("    bake-mcp [FLAGS]\n");
        println!("FLAGS:");
        println!("    -d, --detect, --status    Probe and display installed suite tools in terminal");
        println!("    -h, --help                Print help information");
        println!("    -v, --version             Print version information\n");
        println!("DEFAULT:");
        println!("    Runs the MCP JSON-RPC server on stdio for AI agent connections.");
        return Ok(());
    }

    if args.iter().any(|arg| arg == "--version" || arg == "-v") {
        println!("bake-mcp {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // Initialize MCP server logging (strictly directed to stderr to protect stdio JSON-RPC)
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .try_init();

    tracing::info!("Starting Bake MCP Server (version {})", env!("CARGO_PKG_VERSION"));

    let transport = create_stdio_transport().await;
    let service = McpServer::new()
        .serve(transport)
        .await
        .inspect_err(|e| {
            tracing::error!("Serving error: {:?}", e);
        })?;

    service.waiting().await?;
    Ok(())
}
