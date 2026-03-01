#![allow(missing_docs)]
//! Tests for the Human-In-The-Loop (HITL) prompt module.

use mcp_guard::hitl::{
    prompt_for_approval_with_answer, prompt_for_approval_with_full_mock,
    prompt_for_approval_with_gui_mock,
};

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
    let result =
        prompt_for_approval_with_answer("risky_tool", r#"{"cmd": "rm -rf /"}"#, false);
    assert!(!result, "Expected deny when no approval given");
}

#[test]
fn test_gui_fallback_approve() {
    // No TTY, display available, GUI dialog returns Ok(true) → approved.
    let result = prompt_for_approval_with_gui_mock(
        "gui_tool",
        r#"{"cmd": "echo test"}"#,
        || false, // no TTY
        || true,  // display server available
        |_msg| Ok(true),
    );
    assert!(result, "Expected approval when GUI dialog returns Ok(true)");
}

#[test]
fn test_gui_fallback_deny() {
    // No TTY, display available, GUI dialog returns Ok(false) → denied.
    let result = prompt_for_approval_with_gui_mock(
        "gui_tool",
        r#"{"cmd": "echo test"}"#,
        || false, // no TTY
        || true,  // display server available
        |_msg| Ok(false),
    );
    assert!(!result, "Expected denial when GUI dialog returns Ok(false)");
}

#[test]
fn test_gui_fallback_error() {
    // No TTY, display available, but GUI dialog itself fails → denied.
    let result = prompt_for_approval_with_gui_mock(
        "gui_tool",
        r#"{"cmd": "echo test"}"#,
        || false, // no TTY
        || true,  // display server available
        |_msg| Err(native_dialog::Error::Io(std::io::Error::from(std::io::ErrorKind::Other))),
    );
    assert!(!result, "Expected denial when GUI dialog fails to render");
}

#[test]
fn test_gui_no_display_server() {
    // No TTY, no display server → should fail closed immediately (deny)
    // without even attempting to spawn zenity/kdialog.
    let result = prompt_for_approval_with_gui_mock(
        "headless_tool",
        r#"{"cmd": "echo test"}"#,
        || false,         // no TTY
        || false,         // no display server (headless / SSH / container)
        |_msg| Ok(true), // must NOT be reached
    );
    assert!(!result, "Expected denial when no display server is available");
}

#[test]
fn test_terminal_prompt_approved() {
    // TTY present, prompt returns Ok(true) → approved.
    let result = prompt_for_approval_with_full_mock(
        "terminal_tool",
        r#"{"cmd": "echo test"}"#,
        || true,          // TTY present
        || true,          // display (unused when TTY present)
        |_msg| Ok(true),  // GUI (unused when TTY present)
        |_msg| Ok(true),  // terminal prompt: approved
    );
    assert!(result, "Expected approval when terminal prompt returns Ok(true)");
}

#[test]
fn test_terminal_prompt_denied() {
    // TTY present, prompt returns Ok(false) → denied.
    let result = prompt_for_approval_with_full_mock(
        "terminal_tool",
        r#"{"cmd": "echo test"}"#,
        || true,           // TTY present
        || true,           // display (unused)
        |_msg| Ok(false),  // GUI (unused)
        |_msg| Ok(false),  // terminal prompt: denied
    );
    assert!(!result, "Expected denial when terminal prompt returns Ok(false)");
}

#[test]
fn test_terminal_prompt_error() {
    // TTY present, prompt returns Err → denied.
    let result = prompt_for_approval_with_full_mock(
        "terminal_tool",
        r#"{"cmd": "echo test"}"#,
        || true,  // TTY present
        || true,  // display (unused)
        |_msg| Ok(true), // GUI (unused)
        |_msg| Err(inquire::InquireError::NotTTY), // terminal prompt fails
    );
    assert!(!result, "Expected denial when terminal prompt returns Err");
}
