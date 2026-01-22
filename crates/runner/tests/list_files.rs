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

fn test_bin_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/bin")
}

/// Tests that list_files runs and returns valid JSON.
#[test]
fn test_list_files_runs() {
    let root = project_root();
    let tool_path = test_bin_dir().join("list_files.rs");

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
    let _files: Vec<String> =
        serde_json::from_str(&stdout).expect(&format!("failed to parse JSON output: {}", stdout));
}

/// Tests extracting metadata from the cargo frontmatter header.
#[test]
fn test_list_files_metadata_from_source() {
    let tool_path = test_bin_dir().join("list_files.rs");

    let metadata =
        wasi_runner::extract_from_source(&tool_path).expect("failed to extract metadata");

    assert_eq!(metadata.name, "list_files");
    assert_eq!(metadata.version, "1.0.0");
    assert_eq!(metadata.description, "Lists files in the current directory");
    assert!(metadata.capabilities.read);
    assert!(!metadata.capabilities.write);
    assert!(!metadata.capabilities.net);
}
