# Guard Policy Configuration (`guard-policy.toml`)

The policy file is the brain of `mcp-guard`. It dictates exactly what the LLM (Large Language Model) has access to by inspecting incoming `tools/call` requests. 

You can use either TOML (`.toml`) or YAML (`.yaml`) format. TOML is the standard structure.

## Core Concepts

`mcp-guard` operates as a **Fail Closed** proxy. This means:
*   If a tool is explicitly listed as `allow`, it passes through quickly.
*   If a tool is completely missing from `guard-policy.toml`, it acts suspiciously and defaults to `prompt` (asking the human for terminal approval). 
*   **Background Servers Note**: If you run `mcp-guard` from an IDE like Cursor/Claude Desktop, there is no interactive terminal for you to type "Y". In these cases, tools that default to `prompt` will ultimately **time out or be denied automatically** because you can't interact with the TTY. **Result:** You must explicitly `allow` tools that you intend to use via cursor.

---

## Structure Review

### 1. `[audit]` Section
Controls where the actions get written. 

```toml
[audit]
log_file = "/tmp/mcp-audit.jsonl"
log_level = "info"  # Can configure tracing subscriber (info/debug/trace)
```

The output written to this file looks like:
```json
{"timestamp":"2026-03-01T08:00:00+00:00","direction":"client_to_server","method":"tools/call","tool_name":"execute_command","arguments":{"command":"ls -l"},"action":"approved"}
```

### 2. `[tools.<name>]` Definitions
Every tool block maps a `tool_name` (as requested by the LLM) to a specific rule.

#### `action` Property
The mandatory `action` key accepts:
1. `"allow"`: Permitted silently.
2. `"deny"`: Blocked immediately with a `-32000` JSON-RPC error.
3. `"prompt"`: Pauses execution to show the payload to a local terminal user for interactive approval (Y/n).

#### `deny_patterns` Property (Optional)
If provided, this expects an array of Regex Strings. If the LLM passes *any* argument that satisfies *any* of these regex patterns, the request is instantly denied, **overriding** the `action`.

---

## Complete Example Policy

Below is an extensive, hardened example demonstrating various security postures. 

```toml
[audit]
log_file = "/tmp/mcp-audit.jsonl"
log_level = "info"

# ---------------------------------------------------------
# Safe Read-Only Tools (Allowed silently)
# ---------------------------------------------------------
[tools.list_directory]
action = "allow"

[tools.search_files]
action = "allow"

[tools.read_file]
action = "allow"
# Hard block access to sensitive files even though it is read_file!
deny_patterns = [
    "^/etc/.*",                 # System config
    ".*\\.env.*",               # Environment variables
    ".*id_rsa.*",               # SSH keys
    ".*\\.aws/credentials$",    # AWS credentials
    ".*\\.kube/config$"         # Kubernetes config
]

# ---------------------------------------------------------
# Integration Examples (e.g. Jira / Confluence)
# Because we use these in a background IDE, we MUST `allow` 
# them explicitly or they default to `prompt` (and hang/fail).
# ---------------------------------------------------------
[tools.confluence_search]
action = "allow"

[tools.confluence_search_user]
action = "allow"

[tools.jira_get_all_projects]
action = "allow"

[tools.jira_create_issue]
action = "allow"

# ---------------------------------------------------------
# File Modification Tools (Require HITL approval in a terminal run)
# ---------------------------------------------------------
[tools.write_file]
action = "prompt"

[tools.str_replace_editor]
action = "prompt"

# ---------------------------------------------------------
# Shell & OS Command Execution (High Risk)
# ---------------------------------------------------------
[tools.bash]
action = "prompt"

[tools.execute_command]
action = "prompt"

# ---------------------------------------------------------
# Database Constraints 
# ---------------------------------------------------------
[tools.sql_query]
action = "prompt"
deny_patterns = [
    "(?i).*DROP\\s+(TABLE|DATABASE).*",  # Reject dropping tables
    "(?i).*DELETE\\s+FROM.*",             # Reject raw deletes
    "(?i).*TRUNCATE\\s+TABLE.*"           # Reject truncates
]

# ---------------------------------------------------------
# Explicit Denials
# ---------------------------------------------------------
[tools.danger_tool]
action = "deny"
```
