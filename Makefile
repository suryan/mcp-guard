# mcp-guard — common developer tasks
# Usage: make <target>
# Run from the repository root (or any dir; recipes cd to ROOT).

ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST))))
BIN  := $(ROOT)/target/release/mcp-guard
PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin

.PHONY: help build release install uninstall test check fmt clippy setup-user clean

help: ## Show this help
	@echo "mcp-guard make targets"
	@echo ""
	@grep -E '^[a-zA-Z0-9_-]+:.*?## ' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "Variables: PREFIX=$(PREFIX)"

# ─── build & install ──────────────────────────────────────────────────────────

build: ## Debug build
	cd $(ROOT) && cargo build

release: ## Release build (optimized)
	cd $(ROOT) && cargo build --release

install: release ## Install binary to BINDIR (default: ~/.local/bin)
	install -d "$(BINDIR)"
	install -m 755 "$(BIN)" "$(BINDIR)/mcp-guard"
	@echo "Installed: $(BINDIR)/mcp-guard"
	@echo "Ensure $(BINDIR) is on your PATH (or: make setup-user)"

uninstall: ## Remove binary from BINDIR
	rm -f "$(BINDIR)/mcp-guard"
	@echo "Removed $(BINDIR)/mcp-guard (if present)"

setup-user: install ## Binary + PATH + starter policy + shell rc (macOS/Linux)
	bash "$(ROOT)/scripts/setup-user.sh" --bin "$(BINDIR)/mcp-guard"

# ─── quality ──────────────────────────────────────────────────────────────────

test: ## Run tests
	cd $(ROOT) && cargo test

check: ## Fast typecheck (no binary)
	cd $(ROOT) && cargo check

fmt: ## Format sources
	cd $(ROOT) && cargo fmt

clippy: ## Clippy with -D warnings
	cd $(ROOT) && cargo clippy --all-targets --all-features -- -D warnings

# ─── maintenance ──────────────────────────────────────────────────────────────

clean: ## Remove cargo build artifacts
	cd $(ROOT) && cargo clean
