//! Policy Engine for enforcing security rules on MCP requests.
//!
//! Loads TOML policy configurations, stores rules per-tool,
//! and evaluates requests based on action types and regex patterns.

use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The action to take when a rule matches.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// Allow the tool execution.
    Allow,
    /// Block the tool execution immediately.
    Deny,
    /// Pause and ask the human in the loop for approval.
    Prompt,
}

/// A rule applying to a specific tool or resource name.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolRule {
    /// The default action for this tool.
    pub action: Action,

    /// Optional regex patterns to deny immediately (even if action is allow or prompt).
    #[serde(default)]
    pub deny_patterns: Vec<String>,
}

/// Audit configuration options from the policy.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AuditConfig {
    /// Optional path to the audit log file.
    pub log_file: Option<PathBuf>,

    /// Optional log level (e.g. info, debug).
    pub log_level: Option<String>,
}

/// The top-level policy configuration structure.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Policy {
    /// Audit configuration block.
    #[serde(default)]
    pub audit: AuditConfig,

    /// A map of tool names to their specific rules.
    #[serde(default)]
    pub tools: HashMap<String, ToolRule>,
}

/// Represents the final evaluated decision for a tool request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Evaluation {
    /// Request is allowed.
    Allowed,
    /// Request is denied immediately (either by deny action or via regex hit).
    Denied(String),
    /// Request needs manual approval via TTY prompt.
    PromptRequired,
}

impl Policy {
    /// Loads the policy configuration from a given file path.
    ///
    /// Supports TOML (`.toml`) and YAML (`.yaml` / `.yml`) formats,
    /// determined by the file extension.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read, the extension is unrecognised,
    /// or the content fails to parse.
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)?;
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let policy: Self = match ext.as_str() {
            "yaml" | "yml" => serde_yaml::from_str(&content)
                .map_err(|e| anyhow::anyhow!("Failed to parse YAML policy: {e}"))?,
            _ => toml::from_str(&content)
                .map_err(|e| anyhow::anyhow!("Failed to parse TOML policy: {e}"))?,
        };
        Ok(policy)
    }

    /// Evaluates a tool or resource request against the policy.
    ///
    /// * `tool_name` - The `name` parameter of the MCP request.
    /// * `arguments` - The `arguments` of the MCP request.
    #[must_use]
    pub fn evaluate(&self, tool_name: &str, arguments: Option<&serde_json::Value>) -> Evaluation {
        // Find the specific rule for this tool name.
        if let Some(rule) = self.tools.get(tool_name) {
            // Check if any deny patterns match any string argument
            if let Some(serde_json::Value::Object(map)) = arguments {
                for pattern in &rule.deny_patterns {
                    if let Ok(re) = Regex::new(pattern) {
                        for (_, val) in map {
                            if let Some(val_str) = val.as_str()
                                && re.is_match(val_str)
                            {
                                return Evaluation::Denied(format!(
                                    "Argument matched deny pattern: {pattern}"
                                ));
                            }
                        }
                    }
                }
            }

            // If no immediate denies triggered, follow the specified action.
            match rule.action {
                Action::Allow => Evaluation::Allowed,
                Action::Deny => {
                    Evaluation::Denied("Blocked by explicit deny action in policy.".to_string())
                }
                Action::Prompt => Evaluation::PromptRequired,
            }
        } else {
            // If the tool is not explicitly defined in the policy, we might want to fail open,
            // fail closed, or prompt. We'll default to prompt for unknown tools to be safe,
            // or we could design it to default to allow/deny based on a global config.
            // Leaving as prompt for security unless told otherwise.
            Evaluation::PromptRequired
        }
    }
}
