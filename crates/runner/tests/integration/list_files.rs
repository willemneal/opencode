use super::{project_root, run_tool, test_bin_dir};

#[test]
fn runs() {
    let root = project_root();
    let tool_path = test_bin_dir().join("list_files.rs");

    let output = run_tool(&tool_path, Some(&root), false, &[]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "wasi-runner failed with stderr: {stderr}"
    );

    let _files: Vec<String> = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| panic!("failed to parse JSON output: {stdout}"));
}

#[test]
fn metadata_from_source() {
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
