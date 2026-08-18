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

## ⚡ Install

No release binaries required — build from source on the machine that will run it.
Works on **macOS** and **Linux** (including WSL2).

### One-liner (recommended)

```bash
# Binary + PATH (~/.local/bin) + starter policy
curl -fsSL https://raw.githubusercontent.com/suryan/mcp-guard/main/scripts/install.sh \
  | bash -s -- --setup-user --yes
```

What that does:

1. Bootstraps [rustup](https://rustup.rs) if `cargo` is missing
2. Builds a release binary → `~/.local/bin/mcp-guard`
3. Writes `~/.config/mcp-guard/path.env` and sources it from `~/.bashrc` / `~/.zshrc` / `~/.profile` / `~/.zprofile`
4. Writes a starter policy at `~/.config/mcp-guard/policy.toml` (existing files are left alone)

```bash
# options
curl -fsSL https://raw.githubusercontent.com/suryan/mcp-guard/main/scripts/install.sh -o install-mcp-guard.sh
bash install-mcp-guard.sh --help
bash install-mcp-guard.sh --setup-user --yes
```

Open a **new shell** after install, then:

```bash
which -a mcp-guard
mcp-guard --help
```

### From a local clone

```bash
./scripts/install.sh --local --setup-user
# or stepwise:
make setup-user              # release binary + PATH + starter policy
```

### Rust developers

```bash
cargo install --git https://github.com/suryan/mcp-guard --locked
# then user integration only:
./scripts/setup-user.sh
```

**Requirements:** `git`, a C linker (`build-essential` on Debian/Ubuntu, Xcode CLT on macOS),
network for crates.io on first build.

---

## 🚀 Quick Start

### 1. Wrap your MCP Server
Update your `mcp.json` to use `mcp-guard` as the primary command:

```json
"my-server": {
  "command": "mcp-guard",
  "args": [
    "run",
    "--policy", "/home/you/.config/mcp-guard/policy.toml",
    "uvx",
    "--",
    "my-server-command"
  ]
}
```

Use the absolute policy path from `~/.config/mcp-guard/mcp.json.example` (written by `--setup-user`). MCP hosts do not expand `~`.

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
    "run",
    "--policy", "/home/you/.config/mcp-guard/policy.toml",
    "mcp-secret-launcher",
    "--",
    "run", "--profile", "my-server",
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
