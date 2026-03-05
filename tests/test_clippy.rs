#![allow(clippy::expect_used, clippy::panic)]
//! Integration test to ensure code passes clippy lints.
//! This ensures that `cargo test` fails if `cargo clippy --all-targets --all-features` reports warnings.

use std::process::Command;

#[test]
fn test_clippy_warnings() {
    let output = Command::new("cargo")
        .args([
            "clippy",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ])
        .output()
        .expect("Failed to run cargo clippy");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Clippy check failed.\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}\n\nRun 'cargo clippy --all-targets --all-features' to see details."
        );
    }
}
