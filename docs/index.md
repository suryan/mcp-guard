# MCP Guard Documentation

Welcome to the comprehensive documentation for `mcp-guard`. 

`mcp-guard` is a secure Layer 7 firewall and proxy for the Model Context Protocol (MCP). It intercepts JSON-RPC standard I/O traffic between MCP clients (like Cursor or Claude Desktop) and target servers, enforcing strict access controls and prompting humans for approval on sensitive operations.

## Guides

- [**Configuring Policies (`guard-policy.toml`)**](policy.md)  
  Learn about the core of `mcp-guard`: writing fail-closed access rules, regex restrictions, and audit logging settings.
  
- [**Usage & Integration**](usage.md)  
  Examples of how to run `mcp-guard` via CLI and how to integrate it as a transparent proxy inside your `mcp.json` config. Includes a real-world Atlassian (Jira/Confluence) setup.

- [**System Architecture & Flow**](architecture.md)  
  D2 diagrams showing the internal pipeline of how a `tools/call` JSON request is parsed, evaluated, verified, and logged.
