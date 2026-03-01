# Usage & Integration Guide

`mcp-guard` runs locally and operates entirely on standard I/O (stdin/stdout) bridging. 

## CLI Execution

When evaluating `mcp-guard`, you can run it from a standard terminal. 

The command signature is:
```bash
mcp-guard run --policy <POLICY_PATH> <TARGET_EXECUTABLE> -- <TARGET_ARGS>...
```

For example, to wrap an imaginary python script:
```bash
mcp-guard run --policy ~/.mcp_guard/policy.toml python -- script.py
```

Because this is a TTY terminal, tools set to `action = "prompt"` will pause execution, print the JSON-RPC to your console, and await your `Y/n` keypress.

---

## IDE Integration (Cursor / Claude Desktop)

Most often, you install `mcp-guard` to wrap another MCP Server inside an interactive IDE like Cursor. 

### Important Note on CLI Parser

`mcp-guard` extracts CLI targets using a strict separator (`--`). Everything **before** the `--` belongs to `mcp-guard`. Everything **after** the `--` belongs to the target server execution.
* The mandatory `<TARGET_EXECUTABLE>` must come immediately **before** the `--` separator.
* The `<TARGET_ARGS>` must come immediately **after** the `--` separator.

### Real-World Example: Atlassian Server with `mcp-secret-launcher`

If you are using an IDE, you will modify your `mcp.json` file. Here is a secure, scrubbed real-world implementation wrapping another executable (`mcp-secret-launcher`) that *in turn* kicks off a command (`uvx mcp-atlassian`). 

Notice how `mcp-secret-launcher` represents the target executable (placed above the first `--`), while the arguments for it are passed safely below it. 

#### `mcp.json`
```json
{
  "mcpServers": {
    "mcp-atlassian": {
      "command": "mcp-guard",
      "args": [
        "run",
        "--policy",
        "/home/user/.mcp_guard/guard-policy.toml",
        "mcp-secret-launcher",
        "--",
        "run",
        "--profile",
        "mcp-atlassian",
        "--",
        "uvx",
        "mcp-atlassian"
      ],
      "env": {
        "DBUS_SESSION_BUS_ADDRESS": "unix:path=/run/user/1000/bus",
        "JIRA_URL": "https://company.atlassian.net",
        "JIRA_USERNAME": "jane.doe@example.com",
        "CONFLUENCE_URL": "https://company.atlassian.net/wiki",
        "CONFLUENCE_USERNAME": "jane.doe@example.com"
      },
      "disabled": false,
      "autoApprove": []
    }
  }
}
```

This ensures when Claude or Cursor kicks off the connection, `mcp-guard` boots first, establishes the policy map from `/home/user/.mcp_guard/guard-policy.toml`, and passes the Atlassian credentials directly down to the target process.

*Reminder: Because this connects through an IDE's background process instead of a TTY terminal, tools that trigger a `prompt` will silently fail. Update your policy file to `allow` the tools you want the IDE to process.*
