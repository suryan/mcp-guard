//! Human-In-The-Loop (HITL) interception for high-risk operations.
//!
//! Uses interactive prompts to ask the developer for confirmation
//! before passing payloads to the underlying server.

use inquire::Confirm;
use native_dialog::{MessageDialogBuilder, MessageLevel};

/// Returns `true` if a controlling terminal (`/dev/tty`) is available.
///
/// `inquire` opens `/dev/tty` directly rather than using stdin, so this is
/// the correct pre-flight check to avoid rendering a prompt in headless
/// environments (CI, piped usage, `cargo test`).
fn has_tty() -> bool {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .is_ok()
}

/// Returns `true` if a graphical display server is available to render a
/// native dialog.
///
/// On **Linux / BSD** the native-dialog crate shells out to `zenity` or
/// `kdialog`, both of which require an X11 or Wayland compositing session.
/// We detect this by checking the conventional environment variables set by
/// those sessions before attempting to spawn the subprocess — avoiding a
/// misleading `MissingDep` / I/O error and replacing it with a clear log
/// message that tells the operator how to unblock themselves.
///
/// On **macOS** `native-dialog` uses `AppKit` directly (no external tool, no
/// display env needed), so we always return `true` there.
fn has_display() -> bool {
    // macOS: AppKit is always available when the process has a GUI session.
    #[cfg(target_os = "macos")]
    {
        true
    }
    // Linux / BSD: a display compositor must be reachable.
    #[cfg(not(target_os = "macos"))]
    {
        std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok()
    }
}

fn default_gui_dialog(message: &str) -> native_dialog::Result<bool> {
    MessageDialogBuilder::default()
        .set_title("MCP Guard - Approval Required")
        .set_text(message)
        .set_level(MessageLevel::Warning)
        .confirm()
        .show()
}

/// Renders an interactive confirmation prompt on the current terminal using
/// `inquire`.  Extracted into a named function so coverage tools can track it
/// and tests can stub it via `prompt_for_approval_with_full_mock`.
fn default_terminal_prompt(message: &str) -> inquire::error::InquireResult<bool> {
    Confirm::new(message).with_default(false).prompt()
}

/// Core approval logic.
///
/// `override_answer` is used in tests to inject a predetermined answer without
/// any TTY interaction.  Pass `None` for normal production behaviour.
fn prompt_for_approval_impl(
    tool_name: &str,
    payload: &str,
    override_answer: Option<bool>,
    tty_checker: fn() -> bool,
    display_checker: fn() -> bool,
    gui_dialog_fn: fn(&str) -> native_dialog::Result<bool>,
    terminal_prompt_fn: fn(&str) -> inquire::error::InquireResult<bool>,
) -> bool {
    if let Some(ans) = override_answer {
        tracing::info!(
            "HITL answer injected (test/mock): {} for '{tool_name}'",
            if ans { "approved" } else { "denied" }
        );
        return ans;
    }

    let message = format!(
        "MCP Client is requesting to execute: '{tool_name}'\nPayload: {payload}\nAllow this execution?"
    );

    // Read env-var mock used by integration tests / CI that need to exercise
    // the proxy without a real terminal.
    if let Ok(mock_ans) = std::env::var("MCP_GUARD_MOCK_HITL") {
        let approved = mock_ans == "approve";
        tracing::info!(
            "HITL mock env override: {} for '{tool_name}'",
            if approved { "approved" } else { "denied" }
        );
        return approved;
    }

    if !tty_checker() {
        // No terminal — try a native GUI dialog instead.
        if !display_checker() {
            // No display server is reachable either.  This typically means
            // mcp-guard is running inside a headless SSH session, a container,
            // or WSL without a GUI layer.  Fail closed and tell the operator
            // exactly how to unblock themselves.
            tracing::warn!(
                "HITL: no TTY and no display server available for '{tool_name}'. \
                 Defaulting to deny. \
                 To override, set MCP_GUARD_MOCK_HITL=approve (or =deny) in the \
                 environment before starting your MCP client."
            );
            return false;
        }

        tracing::info!("No TTY detected; attempting GUI dialog for '{tool_name}'");

        // Attempt to show a native dialog.
        match gui_dialog_fn(&message) {
            Ok(true) => {
                tracing::info!("HITL GUI Approved execution of {tool_name}");
                return true;
            }
            Ok(false) => {
                tracing::info!("HITL GUI Denied execution of {tool_name}");
                return false;
            }
            Err(e) => {
                tracing::warn!(
                    "HITL: GUI dialog failed for '{tool_name}': {e}. \
                     Defaulting to deny. \
                     Ensure zenity or kdialog is installed (Linux), or that \
                     the process has GUI access. \
                     Set MCP_GUARD_MOCK_HITL=approve to bypass interactively."
                );
                return false;
            }
        }
    }

    match terminal_prompt_fn(&message) {
        Ok(true) => {
            tracing::info!("HITL Approved execution of {tool_name}");
            true
        }
        Ok(false) => {
            tracing::info!("HITL Denied execution of {tool_name}");
            false
        }
        Err(_) => {
            tracing::warn!("Failed to get HITL response, defaulting to deny");
            false
        }
    }
}

/// Prompts the user to approve or deny a specific tool call.
///
/// * `tool_name` - The name of the tool being called.
/// * `payload` - The JSON-RPC payload representing the arguments.
///
/// Returns `true` if approved, `false` otherwise.
pub fn prompt_for_approval(tool_name: &str, payload: &str) -> bool {
    prompt_for_approval_impl(
        tool_name,
        payload,
        None,
        has_tty,
        has_display,
        default_gui_dialog,
        default_terminal_prompt,
    )
}

/// Test-only entry point that injects a predetermined answer, bypassing all
/// TTY / env-var logic without requiring `unsafe` env mutation.
///
/// # Test only
///
/// This function is public only so that integration tests (which compile as a
/// separate crate) can call it. Do not use it in production code.
#[doc(hidden)]
pub fn prompt_for_approval_with_answer(tool_name: &str, payload: &str, answer: bool) -> bool {
    prompt_for_approval_impl(
        tool_name,
        payload,
        Some(answer),
        has_tty,
        has_display,
        default_gui_dialog,
        default_terminal_prompt,
    )
}

/// Test-only entry point that injects both a TTY mock and a GUI mock,
/// bypassing the OS dialog and display-server check.
#[doc(hidden)]
pub fn prompt_for_approval_with_gui_mock(
    tool_name: &str,
    payload: &str,
    tty_checker: fn() -> bool,
    display_checker: fn() -> bool,
    gui_dialog_fn: fn(&str) -> native_dialog::Result<bool>,
) -> bool {
    prompt_for_approval_impl(
        tool_name,
        payload,
        None,
        tty_checker,
        display_checker,
        gui_dialog_fn,
        default_terminal_prompt,
    )
}

/// Test-only entry point that injects all three OS-interaction functions,
/// allowing full coverage of the terminal-prompt branch without blocking.
#[doc(hidden)]
pub fn prompt_for_approval_with_full_mock(
    tool_name: &str,
    payload: &str,
    tty_checker: fn() -> bool,
    display_checker: fn() -> bool,
    gui_dialog_fn: fn(&str) -> native_dialog::Result<bool>,
    terminal_prompt_fn: fn(&str) -> inquire::error::InquireResult<bool>,
) -> bool {
    prompt_for_approval_impl(
        tool_name,
        payload,
        None,
        tty_checker,
        display_checker,
        gui_dialog_fn,
        terminal_prompt_fn,
    )
}

#[cfg(test)]
mod tests {
    use super::prompt_for_approval_impl;

    /// Exercises the `"approved"` arm of the injected-answer tracing line
    /// directly through `prompt_for_approval_impl` so llvm-cov registers both
    /// branches of the `if ans { … } else { … }` expression.
    #[test]
    fn test_impl_injected_approve_branch() {
        let result = prompt_for_approval_impl(
            "tool",
            "payload",
            Some(true),
            || false,
            || false,
            |_| Ok(false),
            |_| Ok(false),
        );
        assert!(result);
    }
}
