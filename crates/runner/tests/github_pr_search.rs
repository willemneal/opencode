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

#[test]
fn test_github_pr_search_describe() {
    let root = project_root();
    let tool_path = root.join("crates/bin/github_pr_search.rs");

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

    let metadata: serde_json::Value =
        serde_json::from_str(&stdout).expect("failed to parse metadata JSON");

    assert_eq!(metadata["name"], "github_pr_search");
    assert_eq!(metadata["version"], "1.0.0");
    assert!(!metadata["capabilities"]["read"].as_bool().unwrap());
    assert!(!metadata["capabilities"]["write"].as_bool().unwrap());
    assert!(metadata["capabilities"]["net"].as_bool().unwrap());

    let args = metadata["args"].as_array().unwrap();
    let arg_names: Vec<&str> = args.iter().map(|a| a["name"].as_str().unwrap()).collect();
    assert!(arg_names.contains(&"query"));
    assert!(arg_names.contains(&"owner"));
    assert!(arg_names.contains(&"repo"));
    assert!(arg_names.contains(&"limit"));
    assert!(arg_names.contains(&"state"));
}

#[test]
fn test_github_pr_search_no_query() {
    let root = project_root();
    let tool_path = root.join("crates/bin/github_pr_search.rs");

    let output = Command::new("cargo")
        .args([
            "+nightly",
            "run",
            "--bin",
            "wasi-runner",
            "run",
            "--allow-net",
        ])
        .arg(&tool_path)
        .arg("--")
        .current_dir(&root)
        .env_remove("GITHUB_TOKEN")
        .output()
        .expect("failed to execute wasi-runner");

    assert!(!output.status.success());
}

#[test]
fn test_github_pr_search_invalid_state() {
    let root = project_root();
    let tool_path = root.join("crates/bin/github_pr_search.rs");

    let output = Command::new("cargo")
        .args([
            "+nightly",
            "run",
            "--bin",
            "wasi-runner",
            "run",
            "--allow-net",
        ])
        .arg(&tool_path)
        .arg("--")
        .args(["--query", "test", "--state", "invalid"])
        .current_dir(&root)
        .output()
        .expect("failed to execute wasi-runner");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
}

/// Search for the cargo script RFC PR in rust-lang/cargo.
#[test]
fn test_github_pr_search_cargo_script() {
    let root = project_root();
    let tool_path = root.join("crates/bin/github_pr_search.rs");

    let output = Command::new("cargo")
        .args([
            "+nightly",
            "run",
            "--bin",
            "wasi-runner",
            "run",
            "--allow-net",
        ])
        .arg(&tool_path)
        .arg("--")
        .args([
            "--query",
            "cargo script",
            "--owner",
            "rust-lang",
            "--repo",
            "cargo",
            "--state",
            "all",
            "--limit",
            "5",
        ])
        .current_dir(&root)
        .output()
        .expect("failed to execute wasi-runner");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "search failed with stderr: {}",
        stderr
    );

    let result: serde_json::Value =
        serde_json::from_str(&stdout).expect("failed to parse JSON output");

    assert!(
        result["total"].as_u64().unwrap() > 0,
        "expected at least one PR"
    );
    assert!(!result["prs"].as_array().unwrap().is_empty());
}
