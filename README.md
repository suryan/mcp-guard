# 🛡️ MCP Guard

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
[![Platform](https://img.shields.io/badge/platform-linux%20%7C%20macos%20%7C%20windows-lightgrey.svg)]()

**A secure Layer 7 firewall and proxy for the Model Context Protocol (MCP).** Intercept `stdio` traffic, enforce security policies, and support Human-In-The-Loop (HITL) approval workflows to protect your local resources.

---

## 🚀 Overview

`mcp-guard` sits transparently between an MCP Client (like Cursor or Claude Desktop) and an underlying target MCP Server. It parses JSON-RPC messages and evaluates them against rules defined in a local policy file.

### Key Features:
- 🛡️ **Fine-grained Access Control:** Permit or deny specific MCP tools and resources.
- 🔍 **Argument Validation:** Use regex matching to prevent prompt injection or risky parameters.
- 🤝 **Human-In-The-Loop (HITL):** Interactive user confirmation for sensitive tool calls.
- 📋 **Audit Logging:** Comprehensive JSON Lines audit logs for compliance and debugging.

---

## ⚡ Quick Start

### 1. Wrap your MCP Server
Update your `mcp.json` to use `mcp-guard` as the primary command:

```json
"my-server": {
  "command": "mcp-guard",
  "args": [
    "--policy", "guard-policy.toml",
    "--",
    "uvx", "my-server-command"
  ]
}
```

### 2. Configure Your Policy
Create a `guard-policy.toml` file:

```toml
[tools]
# Explicitly allow safe tools
list_files = "allow"

# Require manual approval for sensitive tools
delete_file = "prompt"

# Block tools matching specific patterns
git_push = { action = "deny", deny_patterns = ["--force"] }
```

---

## 🛡️ Defense in Depth

For maximum security, combine **mcp-guard** with [**mcp-secret-launcher**](https://github.com/suryan/mcp-secret-launcher):

- **mcp-guard (Layer 7):** Protects your *resources* by intercepting tool calls and enforcing HITL.
- **mcp-secret-launcher (Layer 3/4):** Protects your *credentials* by keeping them in the OS keyring.

**Complete Security Stack:**
```json
"my-server": {
  "command": "mcp-guard",
  "args": [
    "--policy", "guard-policy.toml",
    "--",
    "mcp-secret-launcher", "run", "--profile", "my-server",
    "--",
    "uvx", "my-server-command"
  ]
}
```

---

## 📖 Learn More

| Guide | Description |
| :--- | :--- |
| [📝 Configuring Policies](docs/policy.md) | Learn how to configure fail-closed access rules and regex patterns. |
| [📂 Usage & Integration](docs/usage.md) | Real-world examples for Cursor, Claude, and more. |
| [🏗️ Architecture](docs/architecture.md) | Component mapping and execution flow pipelines. |
| [👩‍💻 Development Guide](docs/development.md) | Setup, testing, and contribution instructions. |

## ⚖️ License

Distributed under the MIT License. See [LICENSE](LICENSE) for more information.
