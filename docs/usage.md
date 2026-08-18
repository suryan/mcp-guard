# Usage & Integration Guide

`mcp-guard` runs locally and operates entirely on standard I/O (stdin/stdout) bridging.

## Installation

`mcp-guard` is a single Rust binary. Prefer **install from source** on each machine
(no GitHub release artifacts to maintain). Supported: **macOS**, **Linux**, **WSL2**.

### One-liner (recommended)

```bash
# Binary + PATH (~/.local/bin) + starter policy + shell rc
curl -fsSL https://raw.githubusercontent.com/suryan/mcp-guard/main/scripts/install.sh \
  | bash -s -- --setup-user --yes
```

What it does:

1. Ensures a Rust toolchain (`rustup` if `cargo` is missing)
2. Clones/updates the repo under `~/.local/src/mcp-guard` (override with `--dir`)
3. `cargo build --release`
4. Installs to `~/.local/bin/mcp-guard` (override with `--prefix`)
5. With `--setup-user`: runs `scripts/setup-user.sh`
   - writes `~/.config/mcp-guard/path.env` (prepends the bindir)
   - sources it from `~/.bashrc`, `~/.zshrc`, `~/.profile`, `~/.zprofile` (idempotent)
   - writes a starter policy at `~/.config/mcp-guard/policy.toml` if missing
   - writes `~/.config/mcp-guard/mcp.json.example`

```bash
# Common options
bash scripts/install.sh --help
bash scripts/install.sh --prefix ~/.local --setup-user --yes
bash scripts/install.sh --ref main
bash scripts/install.sh --local --setup-user            # current clone only
MCP_GUARD_PREFIX=/usr/local sudo -E bash scripts/install.sh --yes   # system-wide (careful)
```

| Variable / flag | Default | Meaning |
|-----------------|---------|---------|
| `--prefix` / `MCP_GUARD_PREFIX` | `~/.local` | Binary at `$PREFIX/bin/mcp-guard` |
| `--ref` / `MCP_GUARD_REF` | `main` | Git branch, tag, or commit |
| `--repo` / `MCP_GUARD_REPO` | this GitHub repo | Clone URL |
| `--dir` / `MCP_GUARD_DIR` | `~/.local/src/mcp-guard` | Checkout path |
| `--setup-user` | off | PATH + starter policy + shell rc (see `setup-user.sh`) |
| `--no-shell-rc` | off | Write `path.env` / policy only; do not edit rc files |
| `--no-policy` | off | Skip writing a starter policy file |
| `--yes` | off | Non-interactive rustup install |

**Requirements:** `curl` + `git`, C linker (`build-essential` on Debian/Ubuntu,
Xcode CLT on macOS: `xcode-select --install`), network for crates.io.

### `cargo install` (Rust toolchain already present)

```bash
cargo install --git https://github.com/suryan/mcp-guard --locked
./scripts/setup-user.sh    # PATH + starter policy (from a clone)
```

### From a local clone

```bash
git clone https://github.com/suryan/mcp-guard.git
cd mcp-guard
./scripts/install.sh --local --setup-user
# or stepwise:
make setup-user                 # release binary + PATH + starter policy
```

### Make shortcuts

```bash
make help          # list targets
make release       # optimized build
make install       # install binary to ~/.local/bin
make setup-user    # binary + PATH + starter policy + shell rc
make test          # cargo test
```

Open a **new shell** after `--setup-user`, then `which -a mcp-guard`.

MCP / IDE hosts often skip shell profiles — put `~/.local/bin` first on that
host’s `PATH` (or use the absolute path to `mcp-guard`).

---

## CLI Execution

When evaluating `mcp-guard`, you can run it from a standard terminal. 

The command signature is:
```bash
mcp-guard run --policy <POLICY_PATH> <TARGET_EXECUTABLE> -- <TARGET_ARGS>...
```

For example, to wrap an imaginary python script:
```bash
mcp-guard run --policy ~/.config/mcp-guard/policy.toml python -- script.py
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
        "/home/user/.config/mcp-guard/policy.toml",
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
        "DISPLAY": ":0",
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

*Note: When `mcp-guard` is launched by a background IDE process, it has no terminal (TTY) attached. If a tool requires a `prompt`, it will automatically detect the lack of a terminal and fallback to a native graphical dialog box (using `zenity`, `kdialog`, or macOS `AppKit`). On Linux, **you must ensure the `DISPLAY` or `WAYLAND_DISPLAY` variables are passed in the `env` block** (as shown above) so `mcp-guard` knows where to render the dialog.*
