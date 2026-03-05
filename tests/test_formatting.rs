#![allow(clippy::expect_used, clippy::panic)]
//! Integration test to ensure code is properly formatted.
//! This ensures that `cargo test` fails if `cargo fmt -- --check` fails.

use std::process::Command;

#[test]
fn test_code_formatting() {
    let output = Command::new("cargo")
        .arg("fmt")
        .arg("--")
        .arg("--check")
        .output()
        .expect("Failed to run cargo fmt");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Code formatting check failed.\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}\n\nRun 'cargo fmt' to fix."
        );
    }
}
