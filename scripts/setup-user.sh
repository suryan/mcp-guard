#!/usr/bin/env bash
# mcp-guard user setup (macOS + Linux)
#
# Idempotent helper used by install.sh and `make setup-user`:
#   - write ~/.config/mcp-guard/path.env  (prepends the binary bindir)
#   - source path.env from shell rc files (bash + zsh, login + interactive)
#   - write a starter policy if none exists
#   - write an mcp.json wrap example
#
# Usage:
#   ./scripts/setup-user.sh
#   ./scripts/setup-user.sh --no-policy
#   ./scripts/setup-user.sh --no-shell-rc
#   MCP_GUARD_BIN=~/.local/bin/mcp-guard ./scripts/setup-user.sh
#
# Environment:
#   MCP_GUARD_BIN     path to mcp-guard binary (default: first on PATH or ~/.local/bin)
#   MCP_GUARD_PREFIX  used to find $PREFIX/bin/mcp-guard when BIN unset

set -euo pipefail

WRITE_SHELL_RC=1
WRITE_POLICY=1
BIN="${MCP_GUARD_BIN:-}"
PREFIX="${MCP_GUARD_PREFIX:-${HOME}/.local}"

usage() {
  cat <<'EOF'
Usage: setup-user.sh [options]

  Configure PATH integration, a starter policy, and an MCP wrap example.

  --no-policy      Do not write ~/.config/mcp-guard/policy.toml
  --no-shell-rc    Do not modify ~/.bashrc ~/.zshrc ~/.profile ~/.zprofile
  --bin PATH       mcp-guard binary (default: PATH / $MCP_GUARD_PREFIX/bin)
  -h, --help       Show this help
EOF
}

log()  { printf '==> %s\n' "$*"; }
warn() { printf 'warn: %s\n' "$*" >&2; }
die()  { printf 'error: %s\n' "$*" >&2; exit 1; }

while [ $# -gt 0 ]; do
  case "$1" in
    --no-policy)
      WRITE_POLICY=0; shift ;;
    --no-shell-rc)
      WRITE_SHELL_RC=0; shift ;;
    --bin)
      BIN="${2:-}"; shift 2 ;;
    --bin=*)
      BIN="${1#*=}"; shift ;;
    -h|--help)
      usage; exit 0 ;;
    *)
      die "unknown option: $1 (try --help)" ;;
  esac
done

resolve_bin() {
  if [ -n "$BIN" ] && [ -x "$BIN" ]; then
    return 0
  fi
  if command -v mcp-guard >/dev/null 2>&1; then
    BIN="$(command -v mcp-guard)"
    return 0
  fi
  if [ -x "${PREFIX}/bin/mcp-guard" ]; then
    BIN="${PREFIX}/bin/mcp-guard"
    return 0
  fi
  die "mcp-guard not found; install first or pass --bin /path/to/mcp-guard"
}

config_dir() {
  if [ -n "${XDG_CONFIG_HOME:-}" ]; then
    printf '%s/mcp-guard' "$XDG_CONFIG_HOME"
  else
    printf '%s/.config/mcp-guard' "$HOME"
  fi
}

data_dir() {
  if [ -n "${XDG_DATA_HOME:-}" ]; then
    printf '%s/mcp-guard' "$XDG_DATA_HOME"
  else
    printf '%s/.local/share/mcp-guard' "$HOME"
  fi
}

# Marker used for idempotent rc edits (do not change casually).
RC_MARKER_BEGIN="# >>> mcp-guard path >>>"
RC_MARKER_END="# <<< mcp-guard path <<<"

write_path_env() {
  local dir conf bindir
  dir="$(config_dir)"
  conf="${dir}/path.env"
  bindir="$(dirname "$BIN")"
  mkdir -p "$dir"
  cat >"$conf" <<EOF
# mcp-guard binary directory — keep ahead of other PATH entries.
export PATH="${bindir}:\${PATH}"
EOF
  log "wrote ${conf}"
  PATH_ENV="$conf"
}

# True if this rc already sources mcp-guard path.env (any marker / prior layout).
rc_already_has_path() {
  local file="$1"
  [ -f "$file" ] || return 1
  grep -qE 'config/mcp-guard/path\.env|mcp-guard path' "$file" 2>/dev/null
}

# Append marker block once per file. Safe on macOS (BSD sed) and GNU.
ensure_rc_snippet() {
  local file="$1"
  local parent
  parent="$(dirname "$file")"
  mkdir -p "$parent"

  if rc_already_has_path "$file"; then
    log "shell rc already configured: ${file}"
    return 0
  fi

  if [ ! -f "$file" ]; then
    touch "$file"
  fi

  {
    printf '\n%s\n' "$RC_MARKER_BEGIN"
    printf '[ -f "%s" ] && . "%s"\n' "$PATH_ENV" "$PATH_ENV"
    printf '%s\n' "$RC_MARKER_END"
  } >>"$file"
  log "updated shell rc: ${file}"
}

setup_shell_rc() {
  [ "$WRITE_SHELL_RC" -eq 1 ] || {
    log "skipping shell rc edits (--no-shell-rc)"
    return 0
  }

  ensure_rc_snippet "${HOME}/.bashrc"
  ensure_rc_snippet "${HOME}/.zshrc"
  ensure_rc_snippet "${HOME}/.profile"
  ensure_rc_snippet "${HOME}/.zprofile"
}

script_repo_root() {
  cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd
}

write_starter_policy() {
  [ "$WRITE_POLICY" -eq 1 ] || {
    log "skipping starter policy (--no-policy)"
    return 0
  }

  local dir dest data audit src
  dir="$(config_dir)"
  dest="${dir}/policy.toml"
  data="$(data_dir)"
  audit="${data}/audit.jsonl"
  mkdir -p "$dir" "$data"

  if [ -f "$dest" ]; then
    log "keeping existing policy: ${dest}"
    return 0
  fi

  src="$(script_repo_root)/examples/policy.toml"
  if [ -f "$src" ]; then
    # Rewrite log_file to this user's data dir; leave the rest intact.
    if grep -q '^log_file' "$src"; then
      sed "s|^log_file *=.*|log_file = \"${audit}\"|" "$src" >"$dest"
    else
      {
        printf '[audit]\nlog_file = "%s"\nlog_level = "info"\n\n' "$audit"
        cat "$src"
      } >"$dest"
    fi
  else
    cat >"$dest" <<EOF
# mcp-guard starter policy (fail-closed-ish: unknown tools → prompt).
# Edit this file, then point --policy at it from your MCP host config.
# See docs/policy.md.

[audit]
log_file = "${audit}"
log_level = "info"

# [tools.list_directory]
# action = "allow"
#
# [tools.read_file]
# action = "allow"
# deny_patterns = [
#     "^/etc/.*",
#     ".*\\\\.env.*",
# ]
#
# [tools.execute_command]
# action = "prompt"
EOF
  fi
  log "wrote starter policy: ${dest}"
}

write_wrap_example() {
  local dir dest policy
  dir="$(config_dir)"
  dest="${dir}/mcp.json.example"
  policy="${dir}/policy.toml"
  mkdir -p "$dir"
  cat >"$dest" <<EOF
{
  "mcpServers": {
    "my-server": {
      "command": "mcp-guard",
      "args": [
        "run",
        "--policy",
        "${policy}",
        "uvx",
        "--",
        "my-server-command"
      ]
    }
  }
}
EOF
  log "wrote wrap example: ${dest}"
}

print_summary() {
  local bindir policy
  bindir="$(dirname "$BIN")"
  policy="$(config_dir)/policy.toml"
  cat <<EOF

User setup complete.

  Binary:     ${BIN}
  PATH file:  $(config_dir)/path.env
  Policy:     ${policy}
  Wrap ex.:   $(config_dir)/mcp.json.example

Open a new shell (or: source $(config_dir)/path.env), then:

  which -a mcp-guard
  mcp-guard --help

Wrap an MCP server (see $(config_dir)/mcp.json.example):

  mcp-guard run --policy ${policy} <target> -- <target-args>

MCP / IDE hosts often skip shell rc — set PATH in the host env:

  PATH="${bindir}:\$PATH"

EOF
}

main() {
  log "mcp-guard user setup (macOS/Linux)"
  resolve_bin
  log "using ${BIN} ($("$BIN" --version 2>/dev/null || echo unknown))"
  write_path_env
  write_starter_policy
  write_wrap_example
  setup_shell_rc
  print_summary
}

main
