#![allow(missing_docs)]
use mcp_guard::rpc::{JsonRpcMessage, McpRequestParams};

#[test]
fn test_rpc_parsing() {
    let raw = r#"{
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "execute",
            "arguments": { "cmd": "ls" }
        }
    }"#;

    let msg = JsonRpcMessage::parse(raw.as_bytes()).unwrap();
    assert!(msg.is_tool_call());
    assert!(!msg.is_resource_read());

    let params: McpRequestParams = msg.extract_mcp_params().expect("Should extract params");
    assert_eq!(params.name, "execute");
    assert_eq!(
        params
            .arguments
            .unwrap()
            .get("cmd")
            .unwrap()
            .as_str()
            .unwrap(),
        "ls"
    );
}

#[test]
fn test_error_response_creation() {
    let raw = r#"{
        "jsonrpc": "2.0",
        "id": 42,
        "method": "tools/call",
        "params": {
            "name": "drop_db"
        }
    }"#;

    let msg = JsonRpcMessage::parse(raw.as_bytes()).unwrap();
    let err_msg = msg.create_error_response(-32000, "Blocked");

    assert_eq!(err_msg.jsonrpc, "2.0");
    assert_eq!(err_msg.id.unwrap().as_i64().unwrap(), 42);
    assert!(err_msg.method.is_none());
    assert!(err_msg.params.is_none());
    assert!(err_msg.result.is_none());

    let err_obj = err_msg.error.unwrap();
    assert_eq!(err_obj.get("code").unwrap().as_i64().unwrap(), -32000);
    assert_eq!(err_obj.get("message").unwrap().as_str().unwrap(), "Blocked");
}
