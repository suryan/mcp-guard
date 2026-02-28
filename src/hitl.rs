//! Human-In-The-Loop (HITL) interception for high-risk operations.
//!
//! Uses interactive prompts to ask the developer for confirmation
//! before passing payloads to the underlying server.

use inquire::Confirm;

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

/// Core approval logic.
///
/// `override_answer` is used in tests to inject a predetermined answer without
/// any TTY interaction.  Pass `None` for normal production behaviour.
fn prompt_for_approval_impl(tool_name: &str, payload: &str, override_answer: Option<bool>) -> bool {
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

    if !has_tty() {
        // No controlling terminal — skip the prompt entirely so nothing is
        // printed during headless test runs, piped usage, or CI.
        tracing::warn!("No TTY detected; defaulting HITL to deny for '{tool_name}'");
        return false;
    }

    match Confirm::new(&message).with_default(false).prompt() {
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
    prompt_for_approval_impl(tool_name, payload, None)
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
    prompt_for_approval_impl(tool_name, payload, Some(answer))
}
