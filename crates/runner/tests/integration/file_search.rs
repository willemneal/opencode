use super::{describe_tool, project_root, run_tool};

#[test]
fn with_pattern() {
    let root = project_root();
    let tool_path = root.join("crates/bin/file_search.rs");

    let output = run_tool(
        &tool_path,
        Some(&root),
        false,
        &[
            "--pattern",
            "**/*.rs",
            "--directory",
            root.to_str().unwrap(),
        ],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "wasi-runner failed with stderr: {}",
        stderr
    );

    let files: Vec<String> =
        serde_json::from_str(&stdout).expect(&format!("failed to parse JSON output: {}", stdout));

    for file in &files {
        assert!(file.ends_with(".rs"), "expected .rs file, got: {}", file);
    }
}

#[test]
fn describe() {
    let root = project_root();
    let tool_path = root.join("crates/bin/file_search.rs");

    let output = describe_tool(&tool_path);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "wasi-runner describe failed with stderr: {}",
        stderr
    );

    let metadata: serde_json::Value =
        serde_json::from_str(&stdout).expect("failed to parse metadata JSON");

    assert_eq!(metadata["name"], "file_search");
    assert_eq!(metadata["version"], "1.0.0");
    assert!(metadata["capabilities"]["read"].as_bool().unwrap());
    assert!(!metadata["capabilities"]["write"].as_bool().unwrap());
    assert!(!metadata["capabilities"]["net"].as_bool().unwrap());
}

#[test]
fn invalid_directory() {
    let root = project_root();
    let tool_path = root.join("crates/bin/file_search.rs");

    let output = run_tool(
        &tool_path,
        Some(&root),
        false,
        &["--pattern", "*.rs", "--directory", "/nonexistent/path"],
    );

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(101));
}

#[test]
fn no_pattern() {
    let root = project_root();
    let tool_path = root.join("crates/bin/file_search.rs");

    let output = run_tool(&tool_path, Some(&root), false, &[]);

    assert!(!output.status.success());
}
