use std::io::Write;
use tempfile::NamedTempFile;
use mcp_guard::policy::{Action, Evaluation, Policy, ToolRule};

#[test]
fn test_policy_evaluation() {
    let mut policy = Policy::default();
    policy.tools.insert(
        "read_file".to_string(),
        ToolRule {
            action: Action::Allow,
            deny_patterns: vec!["^/etc/.*".to_string(), ".*\\.env$".to_string()],
        },
    );
    policy.tools.insert(
        "drop_table".to_string(),
        ToolRule {
            action: Action::Deny,
            deny_patterns: vec![],
        },
    );
    policy.tools.insert(
        "commit".to_string(),
        ToolRule {
            action: Action::Prompt,
            deny_patterns: vec![],
        },
    );

    // Allowed rule
    let args1 = serde_json::json!({"path": "/home/user/document.txt"});
    assert_eq!(
        policy.evaluate("read_file", Some(&args1)),
        Evaluation::Allowed
    );

    // Denied by regex
    let args2 = serde_json::json!({"path": "/etc/passwd"});
    match policy.evaluate("read_file", Some(&args2)) {
        Evaluation::Denied(msg) => assert!(msg.contains("deny pattern")),
        _ => panic!("Should be denied by regex"),
    }

    // Explicit Deny
    let args3 = serde_json::json!({});
    match policy.evaluate("drop_table", Some(&args3)) {
        Evaluation::Denied(msg) => assert!(msg.contains("explicit deny action")),
        _ => panic!("Should be explicitly denied"),
    }

    // Prompt
    let args4 = serde_json::json!({"message": "update"});
    assert_eq!(
        policy.evaluate("commit", Some(&args4)),
        Evaluation::PromptRequired
    );

    // Unknown - defaults to Prompt
    let args5 = serde_json::json!({});
    assert_eq!(
        policy.evaluate("unknown_tool", Some(&args5)),
        Evaluation::PromptRequired
    );

    // Regex fallthrough - pattern doesn't match
    let args6 = serde_json::json!({"path": "/home/user/safe.txt"});
    assert_eq!(
        policy.evaluate("read_file", Some(&args6)),
        Evaluation::Allowed
    );

    // Regex fallthrough - invalid regex pattern
    let mut bad_regex_policy = Policy::default();
    bad_regex_policy.tools.insert(
        "read_file".to_string(),
        ToolRule {
            action: Action::Allow,
            deny_patterns: vec!["[unterminated".to_string()],
        },
    );
    assert_eq!(
        bad_regex_policy.evaluate("read_file", Some(&args6)),
        Evaluation::Allowed
    );

    // Regex match on second property to exhaust map iterations
    let mut multi_regex_policy = Policy::default();
    multi_regex_policy.tools.insert(
        "test_tool".to_string(),
        ToolRule {
            action: Action::Allow,
            deny_patterns: vec!["fail".to_string(), "drop".to_string()],
        },
    );
    let args_multi = serde_json::json!({"a": "safe", "b": 123, "c": "drop"});
    match multi_regex_policy.evaluate("test_tool", Some(&args_multi)) {
        Evaluation::Denied(msg) => assert!(msg.contains("deny pattern")),
        _ => panic!("Should deny"),
    }

    // Argument is not an object (Array instead of Map)
    let args_array = serde_json::json!(["array_val"]);
    assert_eq!(
        multi_regex_policy.evaluate("test_tool", Some(&args_array)),
        Evaluation::Allowed
    );
}



#[test]
fn test_policy_load() {
    let mut temp_file = NamedTempFile::new().unwrap();
    let toml_content = r#"
[audit]
log_file = "/tmp/mcp-guard-audit.jsonl"

[tools.read_file]
action = "allow"
deny_patterns = ["^/etc/.*", ".*\\.env$"]

[tools.drop_table]
action = "deny"
"#;
    temp_file.write_all(toml_content.as_bytes()).unwrap();

    let policy = Policy::load(temp_file.path()).unwrap();
    
    assert_eq!(policy.audit.log_file.unwrap().to_str().unwrap(), "/tmp/mcp-guard-audit.jsonl");
    assert_eq!(policy.tools.len(), 2);
    
    let read_file_rule = policy.tools.get("read_file").unwrap();
    assert_eq!(read_file_rule.action, Action::Allow);
    assert_eq!(read_file_rule.deny_patterns.len(), 2);
    
    let drop_table_rule = policy.tools.get("drop_table").unwrap();
    assert_eq!(drop_table_rule.action, Action::Deny);
}

#[test]
fn test_policy_load_yaml() {
    use std::ffi::OsStr;
    // NamedTempFile can't easily get a .yaml extension; use a real named temp path.
    let dir = tempfile::tempdir().unwrap();
    let yaml_path = dir.path().join("policy.yaml");
    let yaml_content = r#"
audit:
  log_file: "/tmp/mcp-guard-audit.jsonl"

tools:
  read_file:
    action: allow
    deny_patterns:
      - "^/etc/.*"
      - ".*\\.env$"
  drop_table:
    action: deny
    deny_patterns: []
"#;
    std::fs::write(&yaml_path, yaml_content).unwrap();

    let policy = Policy::load(&yaml_path).unwrap();

    assert_eq!(
        yaml_path.extension(),
        Some(OsStr::new("yaml")),
        "path should have .yaml extension"
    );
    assert_eq!(policy.audit.log_file.unwrap().to_str().unwrap(), "/tmp/mcp-guard-audit.jsonl");
    assert_eq!(policy.tools.len(), 2);

    let read_file_rule = policy.tools.get("read_file").unwrap();
    assert_eq!(read_file_rule.action, Action::Allow);
    assert_eq!(read_file_rule.deny_patterns.len(), 2);

    let drop_table_rule = policy.tools.get("drop_table").unwrap();
    assert_eq!(drop_table_rule.action, Action::Deny);
}

#[test]
fn test_policy_load_yml_extension() {
    let dir = tempfile::tempdir().unwrap();
    let yml_path = dir.path().join("policy.yml");
    let yaml_content = "tools:\n  safe_tool:\n    action: allow\n    deny_patterns: []\n";
    std::fs::write(&yml_path, yaml_content).unwrap();

    let policy = Policy::load(&yml_path).unwrap();
    assert_eq!(policy.tools.len(), 1);
    let rule = policy.tools.get("safe_tool").unwrap();
    assert_eq!(rule.action, Action::Allow);
}

#[test]
fn test_policy_load_invalid_yaml() {
    // Covers the YAML map_err closure in Policy::load()
    let dir = tempfile::tempdir().unwrap();
    let yaml_path = dir.path().join("bad.yaml");
    std::fs::write(&yaml_path, "tools: [invalid: yaml: content: :::\n").unwrap();

    let result = Policy::load(&yaml_path);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("Failed to parse YAML policy"), "Expected YAML error, got: {msg}");
}

#[test]
fn test_policy_load_invalid_toml() {
    // Covers the TOML map_err closure in Policy::load()
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(b"this is not valid toml !!! @@@\n").unwrap();

    let result = Policy::load(temp_file.path());
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("Failed to parse TOML policy"), "Expected TOML error, got: {msg}");
}

#[test]
fn test_policy_load_nonexistent_file() {
    // Covers the read_to_string ? error path in Policy::load()
    let result = Policy::load("/tmp/mcp-guard-does-not-exist-xyz123.toml");
    assert!(result.is_err());
}
