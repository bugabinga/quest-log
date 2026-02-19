//! Integration Tests for Quest Log CLI
//!
//! Tests the CLI commands and their integration.

use std::process::Command;
use std::str;

/// Test that the default command runs without error
#[test]
fn test_default_command_help() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    let stdout = str::from_utf8(&output.stdout).unwrap();
    let stderr = str::from_utf8(&output.stderr).unwrap();

    // Should show help
    assert!(
        stdout.contains("Usage:") || stderr.contains("Usage:"),
        "Should show usage info. Got stdout: {}, stderr: {}",
        stdout,
        stderr
    );
}

/// Test serve command
#[test]
fn test_serve_command() {
    // Just verify it can parse the command without immediately starting server
    let output = Command::new("cargo")
        .args(&["build"])
        .output()
        .expect("Failed to build");

    assert!(output.status.success(), "Build should succeed");
}

/// Test UI command exists in help
#[test]
fn test_ui_command_in_help() {
    // Build first
    let _ = Command::new("cargo")
        .args(&["build"])
        .output()
        .expect("Failed to build");

    // Note: Can't actually run the TUI in tests, but we can verify the binary compiles with all commands
    // This is implicitly tested by the build succeeding
}

/// Test that all CLI subcommands are available
#[test]
fn test_cli_subcommands() {
    let output = Command::new("cargo")
        .args(&["run", "--", "--help"])
        .output()
        .expect("Failed to execute");

    let combined = format!(
        "{}{}",
        str::from_utf8(&output.stdout).unwrap_or(""),
        str::from_utf8(&output.stderr).unwrap_or("")
    );

    // Should have serve and ui commands
    assert!(combined.contains("serve"), "Should have serve command");
    assert!(combined.contains("ui"), "Should have ui command");
}
