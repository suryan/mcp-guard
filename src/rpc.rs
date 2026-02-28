//! JSON-RPC 2.0 parsing and serialization tailored for MCP.
//!
//! This module provides structures to parse and inspect MCP messages,
//! specifically identifying `CallToolRequest` and `ReadResourceRequest`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents a generic JSON-RPC 2.0 message payload.
/// We use permissive deserialization to selectively inspect only what we care about.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JsonRpcMessage {
    /// The protocol version, typically "2.0".
    #[serde(default)]
    pub jsonrpc: String,

    /// Optional request identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,

    /// The method name, if this is a request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,

    /// The parameters of the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,

    /// The result payload, if this is a success response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,

    /// The error payload, if this is an error response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}

/// Helper struct for deserializing tool or resource interaction parameters.
#[derive(Debug, Clone, Deserialize)]
pub struct McpRequestParams {
    /// The name of the tool or resource.
    pub name: String,

    /// The arguments passed to the tool or resource (can be mapped directly to a JSON object).
    #[serde(default)]
    pub arguments: Option<Value>,
}

impl JsonRpcMessage {
    /// Parses a JSON byte slice into a JSON-RPC message.
    ///
    /// # Errors
    /// Returns an error if the payload is completely malformed JSON.
    pub fn parse(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// Checks if this message represents an MCP tool execution request.
    #[must_use]
    pub fn is_tool_call(&self) -> bool {
        self.method.as_deref() == Some("tools/call")
    }

    /// Checks if this message represents an MCP resource read request.
    #[must_use]
    pub fn is_resource_read(&self) -> bool {
        self.method.as_deref() == Some("resources/read")
    }

    /// Extracts the inner parameters if they match the expected shape of an MCP request.
    #[must_use]
    pub fn extract_mcp_params(&self) -> Option<McpRequestParams> {
        self.params
            .as_ref()
            .and_then(|p| serde_json::from_value(p.clone()).ok())
    }

    /// Creates a JSON-RPC error response indicating that the action was blocked.
    #[must_use]
    pub fn create_error_response(&self, error_code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: self.id.clone(),
            method: None,
            params: None,
            result: None,
            error: Some(serde_json::json!({
                "code": error_code,
                "message": message
            })),
        }
    }
}
