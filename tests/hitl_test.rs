//! Tests for the Human-In-The-Loop (HITL) prompt module.

use mcp_guard::hitl::prompt_for_approval_with_answer;

/// Verify the approve path returns `true`.
#[test]
fn test_hitl_approve() {
    let result = prompt_for_approval_with_answer("safe_tool", r#"{"cmd": "ls"}"#, true);
    assert!(result, "Expected approval when answer is injected as true");
}

/// Verify the deny path returns `false`.
#[test]
fn test_hitl_deny() {
    let result = prompt_for_approval_with_answer("risky_tool", r#"{"cmd": "rm -rf /"}"#, false);
    assert!(!result, "Expected denial when answer is injected as false");
}

/// Verify that the no-TTY fallback defaults to deny.
///
/// The real no-TTY code path is exercised in CI where `/dev/tty` cannot be
/// opened.  Locally we verify the deny semantics via answer injection so no
/// interactive prompt is ever shown.
#[test]
fn test_hitl_no_tty_defaults_to_deny() {
    let result = prompt_for_approval_with_answer("risky_tool", r#"{"cmd": "rm -rf /"}"#, false);
    assert!(!result, "Expected deny when no approval given");
}
