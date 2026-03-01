#![allow(missing_docs)]
#![allow(deprecated)] // assert_cmd::Command::cargo_bin is deprecated in newer versions
use assert_cmd::Command;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_cli_audit_no_args() {
    // Without --log-file, audit should exit with error and print usage hint
    let mut cmd = Command::cargo_bin("mcp-guard").unwrap();
    cmd.arg("audit");
    cmd.assert()
        .failure()
        .stderr(predicates::str::contains("--log-file"));
}

#[test]
fn test_cli_audit_with_log_file() {
    use std::io::Write;

    // Write a known JSONL entry
    let mut temp = NamedTempFile::new().unwrap();
    writeln!(
        temp,
        r#"{{"timestamp":"2026-03-01T08:00:00Z","direction":"client_to_server","method":"tools/call","tool_name":"execute_command","arguments":{{"cmd":"ls"}},"action":"denied"}}"#
    ).unwrap();

    let mut cmd = Command::cargo_bin("mcp-guard").unwrap();
    cmd.arg("audit").arg("--log-file").arg(temp.path());

    cmd.assert()
        .success()
        .stdout(predicates::str::contains("execute_command"))
        .stdout(predicates::str::contains("denied"))
        .stdout(predicates::str::contains("tools/call"));
}

#[test]
fn test_cli_audit_empty_log() {
    let temp = NamedTempFile::new().unwrap(); // empty file

    let mut cmd = Command::cargo_bin("mcp-guard").unwrap();
    cmd.arg("audit").arg("--log-file").arg(temp.path());

    cmd.assert()
        .success()
        .stdout(predicates::str::contains("empty"));
}

#[test]
fn test_proxy_allowed() {
    let mut temp_policy = NamedTempFile::new().unwrap();
    let toml_content = r#"
[tools.allowed_tool]
action = "allow"
"#;
    temp_policy.write_all(toml_content.as_bytes()).unwrap();

    let mut cmd = Command::cargo_bin("mcp-guard").unwrap();
    cmd.arg("run")
        .arg("--policy")
        .arg(temp_policy.path())
        .arg("sh")
        .arg("--")
        .arg("-c")
        .arg(r#"echo "Target starting" >&2; read line; echo '{"jsonrpc": "2.0", "id": 1, "result": {"content": "ok"}}'"#);

    let payload = r#"{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "allowed_tool", "arguments": {}}}"#;

    cmd.write_stdin(format!("{}\n", payload))
        .assert()
        .success()
        .stdout(predicates::str::contains(
            r#"{"jsonrpc": "2.0", "id": 1, "result": {"content": "ok"}}"#,
        ));
}

#[test]
fn test_proxy_denied() {
    let mut temp_policy = NamedTempFile::new().unwrap();
    let toml_content = r#"
[tools.denied_tool]
action = "deny"
"#;
    temp_policy.write_all(toml_content.as_bytes()).unwrap();

    let mut cmd = Command::cargo_bin("mcp-guard").unwrap();
    cmd.arg("run")
        .arg("--policy")
        .arg(temp_policy.path())
        .arg("echo"); // dummy target, doesn't matter because request is blocked

    let payload = r#"{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "denied_tool", "arguments": {}}}"#;

    cmd.write_stdin(format!("{}\n", payload))
        .assert()
        .success() // proxy itself succeeds because it blocks and then the stream closes
        .stdout(predicates::str::contains("Blocked by MCP Guard Policy"));
}

#[test]
fn test_proxy_prompt_approve() {
    let mut temp_policy = NamedTempFile::new().unwrap();
    let toml_content = r#"
[tools.risky_tool]
action = "prompt"
"#;
    temp_policy.write_all(toml_content.as_bytes()).unwrap();

    let mut cmd = Command::cargo_bin("mcp-guard").unwrap();
    cmd.env("MCP_GUARD_MOCK_HITL", "approve")
        .arg("run")
        .arg("--policy")
        .arg(temp_policy.path())
        .arg("sh")
        .arg("--")
        .arg("-c")
        .arg(r#"read line; echo '{"jsonrpc": "2.0", "id": 2, "result": {"content": "ok"}}'"#);

    let payload = r#"{"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "risky_tool", "arguments": {}}}"#;

    // Send payload, then 'y' for the prompt
    cmd.write_stdin(format!("{}\ny\n", payload))
        .assert()
        .success()
        .stdout(predicates::str::contains(
            r#"{"jsonrpc": "2.0", "id": 2, "result": {"content": "ok"}}"#,
        ));
}

#[test]
fn test_proxy_prompt_deny() {
    let mut temp_policy = NamedTempFile::new().unwrap();
    let toml_content = r#"
[tools.risky_tool]
action = "prompt"
"#;
    temp_policy.write_all(toml_content.as_bytes()).unwrap();

    let mut cmd = Command::cargo_bin("mcp-guard").unwrap();
    cmd.env("MCP_GUARD_MOCK_HITL", "deny")
        .arg("run")
        .arg("--policy")
        .arg(temp_policy.path())
        .arg("echo");

    let payload = r#"{"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "risky_tool", "arguments": {}}}"#;

    // Send payload, then 'n' for the prompt
    cmd.write_stdin(format!("{}\nn\n", payload))
        .assert()
        .success()
        .stdout(predicates::str::contains("Blocked by MCP Guard Policy"));
}

#[test]
fn test_proxy_invalid_mcp_request() {
    let mut temp_policy = NamedTempFile::new().unwrap();
    temp_policy
        .write_all(b"[tools.allowed_tool]\naction = \"allow\"\n")
        .unwrap();

    let mut cmd = Command::cargo_bin("mcp-guard").unwrap();
    cmd.arg("run")
        .arg("--policy")
        .arg(temp_policy.path())
        .arg("sh")
        .arg("--")
        .arg("-c")
        .arg(r#"echo "Target started" >&2; read line; echo "Got line: $line" >&2; echo '{"jsonrpc": "2.0", "result": "ok"}'"#);

    // Valid JSON-RPC, but missing 'name' in params
    let payload =
        r#"{"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"something": "else"}}"#;

    cmd.write_stdin(format!("{}\n", payload))
        .assert()
        .success()
        .stdout(predicates::str::contains(
            r#"{"jsonrpc": "2.0", "result": "ok"}"#,
        ));
}

#[test]
fn test_cli_audit_unparseable_and_blank_lines() {
    // Covers: blank-line `continue` branch and unparseable `Err(_)` arm in audit output
    let dir = tempfile::tempdir().unwrap();
    let log_path = dir.path().join("audit.jsonl");
    std::fs::write(
        &log_path,
        // valid entry, blank line, unparseable garbage
        "{\"timestamp\":\"2026-03-01T08:00:00Z\",\"direction\":\"c\",\"method\":\"tools/call\",\"tool_name\":\"t\",\"arguments\":null,\"action\":\"allowed\"}\n\nnot valid json at all\n",
    ).unwrap();

    let mut cmd = Command::cargo_bin("mcp-guard").unwrap();
    cmd.arg("audit").arg("--log-file").arg(&log_path);

    cmd.assert()
        .success()
        .stdout(predicates::str::contains(
            "(unparseable) not valid json at all",
        ))
        .stdout(predicates::str::contains("allowed"));
}

#[test]
fn test_cli_audit_missing_log_file() {
    // Covers: std::fs::read_to_string error path when file doesn't exist
    let mut cmd = Command::cargo_bin("mcp-guard").unwrap();
    cmd.arg("audit")
        .arg("--log-file")
        .arg("/tmp/mcp-guard-does-not-exist-xyz123.jsonl");

    cmd.assert()
        .failure()
        .stderr(predicates::str::contains("Failed to read log file"));
}

#[test]
fn test_proxy_run_with_log_level_in_policy() {
    // Covers: the `log_level` Some branch in main.rs run command
    let dir = tempfile::tempdir().unwrap();
    let policy_path = dir.path().join("policy.toml");
    std::fs::write(
        &policy_path,
        "[audit]\nlog_level = \"debug\"\n\n[tools.echo_tool]\naction = \"allow\"\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mcp-guard").unwrap();
    cmd.arg("run").arg("--policy").arg(&policy_path).arg("echo");

    // Just needs to start and not crash — echo will exit immediately
    cmd.write_stdin("").assert().success();
}
