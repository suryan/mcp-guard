# MCP Guard

A secure Layer 7 firewall and proxy for the Model Context Protocol (MCP) that intercepts `stdio` traffic, enforces security policies, and supports Human-In-The-Loop (HITL) approval workflows to protect local resources.

## Overview

`mcp-guard` sits transparently between an MCP Client (like Cursor or Claude Desktop) and an underlying target MCP Server. By acting as a bidirectional standard I/O proxy, it parses JSON-RPC messages on the fly and evaluates them against rules defined in a local policy file (`.toml` or `.yaml`).

It provides:
- Fine-grained access control to MCP tools and resources.
- Granular argument checking via regex matching.
- Interactive user confirmation for sensitive tool calls (HITL).
- Comprehensive asynchronous audit logging in JSON Lines format.

## Documentation Reference

Rather than a massive single page, detailed guides for `mcp-guard` live in the [`docs/`](docs/) directory.

*   [**Configuring Policies (`guard-policy.toml`)**](docs/policy.md): Learn how to configure fail-closed access rules, deny regex patterns, and control audit settings.
*   [**Usage & IDE Integration**](docs/usage.md): Scrubbed, real-world examples of wrapping targeted processes, passing arguments, and connecting internal IDE networks (e.g. Cursor to Jira/Confluence).
*   [**Architecture & Flow Diagrams**](docs/architecture.md): Component mapping and execution flow pipelines to natively understand exactly how payloads are managed.

---

## Setup & Installation

Ensure you have Rust installed. Then, clone the repository and build the binary:

```bash
cargo build --release
```

Place the compiled binary `target/release/mcp-guard` in your `$PATH`.

---

## Testing

The test suite lives in [`tests/`](tests/) and is organised into four integration test files. All tests are run using standard Cargo tooling — no extra scripts required.

### Running All Tests

```bash
cargo test
```

Expected output: **16 tests, 0 failures.**

### Running a Specific Test Suite

Pass the test binary name (i.e. the filename without `.rs`) to `cargo test`:

```bash
# Run only the RPC tests
cargo test --test rpc_test

# Run only the policy tests
cargo test --test policy_test

# Run only the audit tests
cargo test --test audit_test

# Run only the integration tests
cargo test --test integration_test
```

### Running a Single Test by Name

Append the test function name (or a unique substring of it) after the `--` separator:

```bash
# Run one specific test
cargo test --test policy_test -- test_policy_load
cargo test --test integration_test -- test_proxy_denied

# Run all tests whose name contains a substring (e.g. all proxy tests)
cargo test --test integration_test -- test_proxy
```

### Running Tests with Output Visible

By default Cargo captures stdout/stderr. Pass `--nocapture` to see it:

```bash
cargo test --test audit_test -- --nocapture
```

---

### Test Suite Reference

#### `tests/rpc_test.rs` — JSON-RPC Parsing

Unit tests for the `rpc` module, verifying message parsing and error construction.

| Test | Description |
|------|-------------|
| `test_rpc_parsing` | Parses a `tools/call` JSON-RPC request and asserts that `is_tool_call()` returns `true`, `is_resource_read()` returns `false`, and extracted params (`name`, `arguments`) are correct. |
| `test_error_response_creation` | Verifies that `create_error_response(-32000, "Blocked")` produces a well-formed JSON-RPC error object with the correct `id`, `code`, and `message` fields. |

```bash
cargo test --test rpc_test
```

---

#### `tests/policy_test.rs` — Policy Evaluation & Loading

Tests for the `policy` module covering all three policy actions and edge cases.

| Test | Description |
|------|-------------|
| `test_policy_evaluation` | Exercises the full evaluation matrix: `allow`, `deny` (explicit), `deny` (regex match), `prompt`, unknown tool defaults, invalid regex patterns, and arguments that aren't JSON objects. |
| `test_policy_load` | Writes a TOML policy file to a temp path and verifies it parses correctly — checking the audit log path, tool count, actions, and `deny_patterns` length. |

```bash
cargo test --test policy_test
```

---

#### `tests/audit_test.rs` — Async Audit Logging

Async tests (using `tokio`) for the `audit` module covering normal and error paths.

| Test | Description |
|------|-------------|
| `test_audit_logger` | Creates a temp file logger, sends a record, waits for the background writer, then reads the file and validates JSON fields (`action`, `tool_name`, `arguments`). |
| `test_audit_logger_no_file` | Constructs a logger with `None` path and verifies logging a record does not panic (no-op path). |
| `test_audit_logger_open_error_and_send` | Passes an invalid path (`/dev/null/invalid`); the background task exits, and a subsequent `log()` call silently fails when the channel is closed. |
| `test_audit_logger_create_dir` | Passes a nested path (`subdir/audit.log`) inside a temp dir; verifies the logger auto-creates missing parent directories. |
| `test_audit_logger_no_parent` | Passes `/` as the log path (no parent directory); exercises the `None` branch in directory-creation logic. |
| `test_audit_logger_write_fail` | Points the logger at `/dev/full` (Linux device that always returns ENOSPC); verifies the logger handles write errors without panicking. |

```bash
cargo test --test audit_test
```

---

#### `tests/integration_test.rs` — End-to-End Proxy Behaviour

Black-box tests that spawn the `mcp-guard` binary via `assert_cmd` and feed it real JSON-RPC payloads over stdin.

| Test | Description |
|------|-------------|
| `test_cli_audit` | Runs `mcp-guard audit` and asserts the command exits successfully with a stub message. |
| `test_proxy_allowed` | Sends a `tools/call` for a tool configured with `action = "allow"`; asserts the response from the downstream target is forwarded unmodified. |
| `test_proxy_denied` | Sends a `tools/call` for a tool configured with `action = "deny"`; asserts the proxy returns a JSON-RPC error containing `"Blocked by MCP Guard Policy"` without forwarding the request. |
| `test_proxy_prompt_approve` | Sets `MCP_GUARD_MOCK_HITL=approve` and sends a call for a `prompt` tool; asserts the request is forwarded and the target response is returned. |
| `test_proxy_prompt_deny` | Sets `MCP_GUARD_MOCK_HITL=deny` and sends a call for a `prompt` tool; asserts the proxy blocks the request and returns an error. |
| `test_proxy_invalid_mcp_request` | Sends a `tools/call` with missing `name` in `params`; verifies the proxy passes the malformed payload through instead of crashing (fail-open for unrecognised structures). |

```bash
cargo test --test integration_test
```

> **HITL mock**: Integration tests that exercise Human-In-The-Loop prompts set `MCP_GUARD_MOCK_HITL=approve` or `deny` to bypass the interactive `/dev/tty` prompt.

---

### Test Coverage

Coverage is measured using [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov). Install it once, then run:

```bash
cargo llvm-cov --all-features --workspace --summary-only
```

To generate an HTML report:

```bash
cargo llvm-cov --all-features --workspace --open
```

Results as of the latest run (16 tests, all passing):

| File        | Line Coverage | Region Coverage | Function Coverage |
|-------------|:-------------:|:---------------:|:-----------------:|
| `audit.rs`  | 94.44%        | 94.74%          | 100.00%           |
| `hitl.rs`   | 80.00%        | 71.43%          | 100.00%           |
| `main.rs`   | 95.45%        | 87.18%          | 100.00%           |
| `policy.rs` | 100.00%       | 95.12%          | 100.00%           |
| `proxy.rs`  | 97.27%        | 94.39%          | 100.00%           |
| `rpc.rs`    | 100.00%       | 100.00%         | 100.00%           |
| **TOTAL**   | **96.11%**    | **92.97%**      | **100.00%**       |

> **Line coverage exceeds the 95% threshold.** The uncovered regions in `hitl.rs` are terminal I/O error paths that require an interactive TTY and cannot easily be exercised in an automated test environment.

## Security

- **Fails Open on Unrecognized Structures**: If the payload is completely malformed or isn't a restricted method, it is ignored and passed through to prevent breaking handshakes.
- **Fail Closed on Denials**: Strict rules explicitly reject payloads that match restricted categories or regex patterns without crashing the proxy (sends a clean `-32000` JSON-RPC error).
- **HITL Interactive Prompts**: The interactive approval prompt attempts to use `inquire` which targets `/dev/tty` so it doesn't break the parent JSON-RPC `stdio` pipe. If no terminal is available (e.g., ran by an IDE in the background), it automatically falls back to a native graphical dialog box (using macOS AppKit, or Linux `zenity`/`kdialog`). **Note:** On Linux, your background MCP client must pass the `DISPLAY` or `WAYLAND_DISPLAY` environment variables for the dialog fallback to work.

## License

This project is licensed under the MIT License.

