//! Audit Logging for MCP traffic.
//!
//! Appends structured records to a JSON Lines (.jsonl) file asynchronously.

use serde::Serialize;
use std::path::PathBuf;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tracing::error;

/// A record representing a single logged event.
#[derive(Debug, Clone, Serialize)]
pub struct AuditRecord {
    /// Timestamp of the event in RFC3339 format.
    pub timestamp: String,
    /// Direction of traffic (e.g., client_to_server).
    pub direction: String,
    /// The original JSON-RPC method.
    pub method: Option<String>,
    /// The extracted tool or resource name.
    pub tool_name: Option<String>,
    /// The targeted arguments.
    pub arguments: Option<serde_json::Value>,
    /// The action taken (allowed, denied, approved).
    pub action: String,
}

/// A handle to the audit logger, used to send records to the background writer task.
#[derive(Clone)]
pub struct AuditLogger {
    sender: Option<mpsc::Sender<AuditRecord>>,
}

impl AuditLogger {
    /// Initializes a new `AuditLogger`. Spawns a background Tokio task to write
    /// records to the specified log file sequentially.
    #[allow(clippy::unused_async)]
    pub async fn new(log_file: Option<PathBuf>) -> Self {
        if let Some(path) = log_file {
            let (tx, mut rx) = mpsc::channel::<AuditRecord>(1000); // Buffer up to 1000 records

            // Spawn the background writer task
            tokio::spawn(async move {
                // Ensure the parent directory exists
                if let Some(parent) = path.parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }

                let mut file = match OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .await
                {
                    Ok(f) => f,
                    Err(e) => {
                        error!("Failed to open audit log file at {:?}: {}", path, e);
                        return;
                    }
                };

                // NFR-3: Restrict log file permissions to 0600 on Unix to prevent credential leaks.
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let perms = std::fs::Permissions::from_mode(0o600);
                    if let Err(e) = tokio::fs::set_permissions(&path, perms).await {
                        error!("Failed to set audit log file permissions: {}", e);
                    }
                }

                while let Some(record) = rx.recv().await {
                    if let Ok(json_line) = serde_json::to_string(&record) {
                        let line = format!("{json_line}\n");
                        if let Err(e) = file.write_all(line.as_bytes()).await {
                            error!("Failed to write to audit log: {}", e);
                        }
                    }
                }
            });

            Self { sender: Some(tx) }
        } else {
            Self { sender: None }
        }
    }

    /// Logs an event asynchronously.
    pub async fn log(&self, record: AuditRecord) {
        if let Some(sender) = &self.sender
            && let Err(e) = sender.send(record).await
        {
            error!("Failed to enqueue audit record: {}", e);
        }
    }
}
