//! Entrypoint for the mcp-guard binary.
use clap::Parser;
use mcp_guard::audit::AuditLogger;
use mcp_guard::cli::{Cli, Commands};
use mcp_guard::policy::Policy;
use mcp_guard::proxy::run_proxy;
use tracing::error;
use tracing_subscriber::EnvFilter;

/// The main entrypoint.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing (logs go to stderr by default, which is passed through transparently)
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("mcp_guard=info".parse()?))
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            policy: policy_path,
            target_executable,
            target_args,
        } => {
            let policy = Policy::load(&policy_path)?;

            // Apply log_level from audit config if specified and not already overridden by env
            if std::env::var("RUST_LOG").is_err()
                && let Some(ref level) = policy.audit.log_level
            {
                let directive = format!("mcp_guard={level}");
                if let Ok(_parsed) = directive.parse::<tracing_subscriber::filter::Directive>() {
                    tracing::info!(
                        "Policy log_level '{}' noted (set RUST_LOG to override)",
                        level
                    );
                }
            }

            let logger = AuditLogger::new(policy.audit.log_file.clone()).await;

            if let Err(e) = run_proxy(policy, logger, target_executable, target_args).await {
                error!("Proxy execution failed: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Audit { log_file } => {
            let Some(path) = log_file else {
                eprintln!("Error: --log-file <PATH> is required for the audit subcommand.");
                eprintln!("Example: mcp-guard audit --log-file ~/.kiro/logs/mcp-audit.jsonl");
                std::process::exit(1);
            };

            let content = std::fs::read_to_string(&path)
                .map_err(|e| anyhow::anyhow!("Failed to read log file {}: {e}", path.display()))?;

            if content.trim().is_empty() {
                println!("Audit log is empty: {}", path.display());
                return Ok(());
            }

            println!("=== MCP Guard Audit Log: {} ===\n", path.display());
            for (i, line) in content.lines().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<serde_json::Value>(line) {
                    Ok(entry) => {
                        let ts = entry["timestamp"].as_str().unwrap_or("?");
                        let tool = entry["tool_name"].as_str().unwrap_or("?");
                        let action = entry["action"].as_str().unwrap_or("?");
                        let method = entry["method"].as_str().unwrap_or("?");
                        println!("[{i:>4}] {ts}  {action:>8}  {method}({tool})");
                        if let Some(args) = entry.get("arguments")
                            && !args.is_null()
                        {
                            println!("       args: {args}");
                        }
                    }
                    Err(_) => println!("[{i:>4}] (unparseable) {line}"),
                }
            }
        }
    }

    Ok(())
}
