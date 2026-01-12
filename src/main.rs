use axum::routing::{get, post};
use clap::Parser;
use color_eyre::eyre::Result;
use rbx_studio_server::{
    dud_proxy_loop, get_input_commands_handler, get_server_code_handler,
    post_input_command_handler, post_server_code_result_handler, proxy_handler, request_handler,
    response_handler, AppState, RBXStudioServer, STUDIO_PLUGIN_PORT,
};
use rmcp::ServiceExt;
use std::io;
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing_subscriber::{self, EnvFilter};
mod error;
mod install;
mod rbx_studio_server;

/// Kill any existing process using our port to prevent stale server issues.
/// This is necessary because old MCP server processes can linger and cause conflicts.
async fn kill_existing_server_on_port(port: u16) {
    #[cfg(unix)]
    {
        // Use lsof to find process using the port, then kill it
        let output = tokio::process::Command::new("lsof")
            .args(["-ti", &format!(":{}", port)])
            .output()
            .await;

        if let Ok(output) = output {
            if output.status.success() {
                let pids = String::from_utf8_lossy(&output.stdout);
                for pid_str in pids.lines() {
                    if let Ok(pid) = pid_str.trim().parse::<i32>() {
                        // Don't kill ourselves
                        if pid != std::process::id() as i32 {
                            tracing::info!("Killing stale MCP server process with PID: {}", pid);
                            let _ = tokio::process::Command::new("kill")
                                .arg(pid.to_string())
                                .status()
                                .await;
                            // Give it a moment to die
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        }
                    }
                }
            }
        }
    }

    #[cfg(windows)]
    {
        // Use netstat to find process using the port
        let output = tokio::process::Command::new("netstat")
            .args(["-ano"])
            .output()
            .await;

        if let Ok(output) = output {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let port_pattern = format!(":{}", port);

            for line in output_str.lines() {
                if line.contains(&port_pattern) && line.contains("LISTENING") {
                    // Extract PID from the last column
                    if let Some(pid_str) = line.split_whitespace().last() {
                        if let Ok(pid) = pid_str.parse::<u32>() {
                            if pid != std::process::id() {
                                tracing::info!("Killing stale MCP server process with PID: {}", pid);
                                let _ = tokio::process::Command::new("taskkill")
                                    .args(["/F", "/PID", &pid.to_string()])
                                    .status()
                                    .await;
                                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Simple MCP proxy for Roblox Studio
/// Run without arguments to install the plugin
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Run as MCP server on stdio
    #[arg(short, long)]
    stdio: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(io::stderr)
        .with_target(false)
        .with_thread_ids(true)
        .init();

    let args = Args::parse();
    if !args.stdio {
        return install::install().await;
    }

    tracing::debug!("Debug MCP tracing enabled");

    // Kill any stale MCP server process on our port before starting
    kill_existing_server_on_port(STUDIO_PLUGIN_PORT).await;

    let server_state = Arc::new(Mutex::new(AppState::new()));

    let (close_tx, close_rx) = tokio::sync::oneshot::channel();

    let listener =
        tokio::net::TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), STUDIO_PLUGIN_PORT)).await;

    let server_state_clone = Arc::clone(&server_state);
    let server_handle = if let Ok(listener) = listener {
        let app = axum::Router::new()
            .route("/request", get(request_handler))
            .route("/response", post(response_handler))
            .route("/proxy", post(proxy_handler))
            .route("/mcp/input", get(get_input_commands_handler).post(post_input_command_handler))
            .route("/mcp/server_code", get(get_server_code_handler).post(post_server_code_result_handler))
            .with_state(server_state_clone);
        tracing::info!("This MCP instance is HTTP server listening on {STUDIO_PLUGIN_PORT}");
        tokio::spawn(async {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    _ = close_rx.await;
                })
                .await
                .unwrap();
        })
    } else {
        tracing::info!("This MCP instance will use proxy since port is busy");
        tokio::spawn(async move {
            dud_proxy_loop(server_state_clone, close_rx).await;
        })
    };

    // Create an instance of our counter router
    let service = RBXStudioServer::new(Arc::clone(&server_state))
        .serve(rmcp::transport::stdio())
        .await
        .inspect_err(|e| {
            tracing::error!("serving error: {:?}", e);
        })?;
    service.waiting().await?;

    close_tx.send(()).ok();
    tracing::info!("Waiting for web server to gracefully shutdown");
    server_handle.await.ok();
    tracing::info!("Bye!");
    Ok(())
}
