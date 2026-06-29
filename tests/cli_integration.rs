//! Integration tests for CLI command parsing and help output.
#![allow(
    clippy::tests_outside_test_module,
    reason = "Integration tests in tests/ are only compiled during cargo test"
)]

//! Integration Tests for Quest Log CLI
//!
//! Tests the CLI commands and their integration.

use std::process::Command;
use std::str;

fn quest_log_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_quest-log"))
}

/// Test that the default command runs without error
#[test]
fn test_default_command_help() {
    let output = quest_log_command()
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    let stdout = str::from_utf8(&output.stdout).unwrap();
    let stderr = str::from_utf8(&output.stderr).unwrap();

    // Should show help
    assert!(
        stdout.contains("Usage:") || stderr.contains("Usage:"),
        "Should show usage info. Got stdout: {stdout}, stderr: {stderr}"
    );
}

/// Test that all CLI subcommands are available
#[test]
fn test_cli_subcommands() {
    let output = quest_log_command()
        .arg("--help")
        .output()
        .expect("Failed to execute");

    let combined = format!(
        "{}{}",
        str::from_utf8(&output.stdout).unwrap_or(""),
        str::from_utf8(&output.stderr).unwrap_or("")
    );

    // Should have serve, migrate-only, and editor-password commands
    assert!(combined.contains("serve"), "Should have serve command");
    assert!(
        combined.contains("migrate-only"),
        "Should have migrate-only command"
    );
    assert!(
        combined.contains("editor-password"),
        "Should have editor-password command"
    );
}
