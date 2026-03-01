#![allow(missing_docs)]
use mcp_guard::audit::{AuditLogger, AuditRecord};
use std::io::Read;
use std::time::Duration;
use tempfile::NamedTempFile;
use tokio::time::sleep;

#[tokio::test]
async fn test_audit_logger() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_path_buf();

    // Create logger with temp file
    let logger = AuditLogger::new(Some(path.clone())).await;

    // Create a record
    let record = AuditRecord {
        timestamp: "2026-03-01T08:00:00Z".to_string(),
        direction: "client_to_server".to_string(),
        method: Some("test_method".to_string()),
        tool_name: Some("test_tool".to_string()),
        arguments: Some(serde_json::json!({"arg": "val"})),
        action: "allowed".to_string(),
    };

    // Log the record
    logger.log(record).await;

    // Give the background task time to write
    sleep(Duration::from_millis(50)).await;

    // Read the file content
    let mut file = std::fs::File::open(&path).unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();

    // Verify it's not empty and contains expected JSON
    assert!(!content.is_empty(), "Log file should not be empty");
    let logged: serde_json::Value = serde_json::from_str(content.trim()).unwrap();

    assert_eq!(logged["action"], "allowed");
    assert_eq!(logged["tool_name"], "test_tool");
    assert_eq!(logged["arguments"]["arg"], "val");
}

#[tokio::test]
async fn test_audit_logger_no_file() {
    let logger = AuditLogger::new(None).await;

    let record = AuditRecord {
        timestamp: "2026-03-01T08:00:00Z".to_string(),
        direction: "client_to_server".to_string(),
        method: None,
        tool_name: None,
        arguments: None,
        action: "allowed".to_string(),
    };

    // Should not panic or error
    logger.log(record).await;
}

#[tokio::test]
async fn test_audit_logger_open_error_and_send() {
    // A path that definitely cannot be opened
    let logger = AuditLogger::new(Some(std::path::PathBuf::from("/dev/null/invalid"))).await;

    // allow background task to attempt open and exit
    sleep(Duration::from_millis(50)).await;

    let record = AuditRecord {
        timestamp: "2026-03-01T08:00:00Z".to_string(),
        direction: "client_to_server".to_string(),
        method: None,
        tool_name: None,
        arguments: None,
        action: "allowed".to_string(),
    };

    // Sender should fail to enqueue because rx was dropped when task exited
    logger.log(record).await;
}

#[tokio::test]
async fn test_audit_logger_create_dir() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("subdir/audit.log");
    let _logger = AuditLogger::new(Some(path)).await;
    sleep(Duration::from_millis(50)).await;
}

#[tokio::test]
async fn test_audit_logger_no_parent() {
    // path without parent covers None branch
    let _logger = AuditLogger::new(Some(std::path::PathBuf::from("/"))).await;
    sleep(Duration::from_millis(50)).await;
}

#[tokio::test]
async fn test_audit_logger_write_fail() {
    // writing to /dev/full always fails with ENOSPC
    let logger = AuditLogger::new(Some(std::path::PathBuf::from("/dev/full"))).await;

    let record = AuditRecord {
        timestamp: "2026-03-01T08:00:00Z".to_string(),
        direction: "client_to_server".to_string(),
        method: None,
        tool_name: None,
        arguments: None,
        action: "allowed".to_string(),
    };

    logger.log(record).await;
    sleep(Duration::from_millis(50)).await;
}

#[cfg(unix)]
#[tokio::test]
async fn test_audit_log_file_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audit.jsonl");

    let logger = AuditLogger::new(Some(path.clone())).await;

    let record = AuditRecord {
        timestamp: "2026-03-01T08:00:00Z".to_string(),
        direction: "client_to_server".to_string(),
        method: Some("tools/call".to_string()),
        tool_name: Some("test_tool".to_string()),
        arguments: None,
        action: "allowed".to_string(),
    };
    logger.log(record).await;

    // Allow background task to write and set permissions
    sleep(Duration::from_millis(100)).await;

    let metadata = std::fs::metadata(&path).unwrap();
    let mode = metadata.permissions().mode();
    // Mask to lower 9 bits (rwxrwxrwx) — should be 0o600 (rw-------)
    assert_eq!(
        mode & 0o777,
        0o600,
        "Audit log file should have 0600 permissions"
    );
}
