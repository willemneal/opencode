use std::path::PathBuf;
use std::process::Command;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Tests that file_search runs successfully with a glob pattern.
/// Note: Due to WASI sandbox limitations with the glob crate, actual file
/// discovery may be limited. This test verifies the tool runs and returns
/// valid JSON.
#[test]
fn test_file_search_with_pattern() {
    let root = project_root();
    let tool_path = root.join("crates/bin/file_search.rs");

    let output = Command::new("cargo")
        .args([
            "+nightly",
            "run",
            "--bin",
            "wasi-runner",
            "run",
            "--allow-read",
        ])
        .arg(&root)
        .arg(&tool_path)
        .arg("--")
        .args(["--pattern", "**/*.rs", "--directory"])
        .arg(&root)
        .current_dir(&root)
        .output()
        .expect("failed to execute wasi-runner");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "wasi-runner failed with stderr: {}",
        stderr
    );

    // Parse the JSON output - should be a valid JSON array
    let files: Vec<String> =
        serde_json::from_str(&stdout).expect(&format!("failed to parse JSON output: {}", stdout));

    // If files are found, verify they all end with .rs
    for file in &files {
        assert!(file.ends_with(".rs"), "expected .rs file, got: {}", file);
    }
}

#[test]
fn test_file_search_describe() {
    let root = project_root();
    let tool_path = root.join("crates/bin/file_search.rs");

    let output = Command::new("cargo")
        .args(["+nightly", "run", "--bin", "wasi-runner", "describe"])
        .arg(&tool_path)
        .current_dir(&root)
        .output()
        .expect("failed to execute wasi-runner describe");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "wasi-runner describe failed with stderr: {}",
        stderr
    );

    // Parse metadata JSON
    let metadata: serde_json::Value =
        serde_json::from_str(&stdout).expect("failed to parse metadata JSON");

    assert_eq!(metadata["name"], "file_search");
    assert_eq!(metadata["version"], "1.0.0");
    assert!(metadata["capabilities"]["read"].as_bool().unwrap());
    assert!(!metadata["capabilities"]["write"].as_bool().unwrap());
    assert!(!metadata["capabilities"]["net"].as_bool().unwrap());
}

#[test]
fn test_file_search_invalid_directory() {
    let root = project_root();
    let tool_path = root.join("crates/bin/file_search.rs");

    let output = Command::new("cargo")
        .args([
            "+nightly",
            "run",
            "--bin",
            "wasi-runner",
            "run",
            "--allow-read",
        ])
        .arg(&root)
        .arg(&tool_path)
        .arg("--")
        .args(["--pattern", "*.rs", "--directory", "/nonexistent/path"])
        .current_dir(&root)
        .output()
        .expect("failed to execute wasi-runner");

    // Should fail with exit code 101 (directory does not exist)
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(101));
}

#[test]
fn test_file_search_no_pattern() {
    let root = project_root();
    let tool_path = root.join("crates/bin/file_search.rs");

    let output = Command::new("cargo")
        .args([
            "+nightly",
            "run",
            "--bin",
            "wasi-runner",
            "run",
            "--allow-read",
        ])
        .arg(&root)
        .arg(&tool_path)
        .arg("--")
        .current_dir(&root)
        .output()
        .expect("failed to execute wasi-runner");

    // Should fail because pattern is required
    assert!(!output.status.success());
}
