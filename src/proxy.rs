//! Async I/O Proxy Loop
//!
//! Spawns the underlying MCP server and proxies streams between the MCP client
//! and the server, enforcing policies on intercepted messages.

use std::process::Stdio;

use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};
use tracing::{error, info};

use crate::audit::{AuditLogger, AuditRecord};
use crate::hitl::prompt_for_approval;
use crate::policy::{Evaluation, Policy};
use crate::rpc::JsonRpcMessage;

/// Runs the proxy loop given the policy, logger, and target command.
#[allow(clippy::too_many_lines)]
pub async fn run_proxy(
    policy: Policy,
    audit_logger: AuditLogger,
    target_cmd: String,
    target_args: Vec<String>,
) -> Result<()> {
    info!(
        "Spawning target MCP server: {} {:?}",
        target_cmd, target_args
    );

    let mut child = Command::new(&target_cmd)
        .args(&target_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn target MCP server")?;

    let server_stdin = child
        .stdin
        .take()
        .context("Failed to attach to server stdin")?;
    let server_stdout = child
        .stdout
        .take()
        .context("Failed to attach to server stdout")?;
    let server_stderr = child
        .stderr
        .take()
        .context("Failed to attach to server stderr")?;

    let client_stdin = tokio::io::stdin();
    let client_stdout = tokio::io::stdout();
    let mut client_stderr = tokio::io::stderr();

    // 1. Pass-through Server Stderr to Client Stderr
    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(server_stderr);
        let mut line = String::new();
        while let Ok(n) = reader.read_line(&mut line).await {
            if n == 0 {
                break;
            }
            let _ = client_stderr.write_all(line.as_bytes()).await;
            let _ = client_stderr.flush().await;
            line.clear();
        }
    });

    // 2. Proxy Server Stdout to Client Stdout (with optional logging)
    let _logger_clone = audit_logger.clone();
    let stdout_task = tokio::spawn(async move {
        let mut reader = FramedRead::new(server_stdout, LinesCodec::new());
        let mut writer = FramedWrite::new(client_stdout, LinesCodec::new());

        while let Some(Ok(line)) = reader.next().await {
            // Forward directly to client
            if let Err(e) = writer.send(&line).await {
                error!("Failed to forward server stdout to client: {}", e);
                break;
            }

            // Log server response traces (optional, mostly skipping for performance)
            // Can be added if response auditing is necessary
        }
    });

    // 3. Proxy Client Stdin to Server Stdin, Enforcing Policy
    let logger_clone2 = audit_logger.clone();
    let stdin_task = tokio::spawn(async move {
        let mut reader = FramedRead::new(client_stdin, LinesCodec::new());
        let mut writer = FramedWrite::new(server_stdin, LinesCodec::new());
        let mut client_stdout_direct = tokio::io::stdout();

        while let Some(Ok(line)) = reader.next().await {
            let final_line = line.clone();
            let mut is_blocked = false;

            // Attempt to parse and evaluate
            match JsonRpcMessage::parse(line.as_bytes()) {
                Ok(msg) if msg.is_tool_call() || msg.is_resource_read() => {
                    if let Some(params) = msg.extract_mcp_params() {
                        let evaluation = policy.evaluate(&params.name, params.arguments.as_ref());
                        
                        let action_str = match &evaluation {
                            Evaluation::Allowed => "allowed".to_string(),
                            Evaluation::Denied(_) => "denied".to_string(),
                            Evaluation::PromptRequired => {
                                if prompt_for_approval(&params.name, &line) {
                                    "approved".to_string()
                                } else {
                                    "denied".to_string()
                                }
                            }
                        };

                        // Timestamp
                        let ts = chrono::Utc::now().to_rfc3339();

                        logger_clone2
                            .log(AuditRecord {
                                timestamp: ts,
                                direction: "client_to_server".to_string(),
                                method: msg.method.clone(),
                                tool_name: Some(params.name.clone()),
                                arguments: params.arguments.clone(),
                                action: action_str.clone(),
                            })
                            .await;

                        if action_str == "denied" {
                            is_blocked = true;
                            // Synthesize error directly to client stdout
                            let err_msg =
                                msg.create_error_response(-32000, "Blocked by MCP Guard Policy");
                            if let Ok(err_json) = serde_json::to_string(&err_msg) {
                                let err_line = format!("{err_json}\n");
                                let _ = client_stdout_direct.write_all(err_line.as_bytes()).await;
                                let _ = client_stdout_direct.flush().await;
                            }
                        }
                    } else {
                        // Not a tool call or read resource we care about, pass-through
                    }
                }
                _ => {
                    // Pass-through anything that isn't a tool call/resource read cleanly
                }
            }

            // Forward to server if not blocked
            if !is_blocked
                && let Err(e) = writer.send(&final_line).await {
                    error!("Failed to forward client request to server: {}", e);
                    break;
                }
        }
    });

    let _ = tokio::try_join!(stderr_task, stdout_task, stdin_task);

    let status = child.wait().await?;
    info!("Target server exited with status: {}", status);

    Ok(())
}
