mod file_search;
mod github_pr_search;
mod list_files;

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
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/integration/bin")
}

fn run_tool(
    tool_path: &PathBuf,
    allow_read: Option<&PathBuf>,
    allow_net: bool,
    args: &[&str],
) -> std::process::Output {
    let root = project_root();
    let mut cmd = Command::new("cargo");
    cmd.args(["+nightly", "run", "--bin", "wasi-runner", "run"]);

    if let Some(dir) = allow_read {
        cmd.arg("--allow-read").arg(dir);
    }

    if allow_net {
        cmd.arg("--allow-net");
    }

    cmd.arg(tool_path);

    if !args.is_empty() {
        cmd.arg("--");
        cmd.args(args);
    }

    cmd.current_dir(&root)
        .output()
        .expect("failed to execute wasi-runner")
}

fn describe_tool(tool_path: &PathBuf) -> std::process::Output {
    let root = project_root();
    Command::new("cargo")
        .args(["+nightly", "run", "--bin", "wasi-runner", "describe"])
        .arg(tool_path)
        .current_dir(&root)
        .output()
        .expect("failed to execute wasi-runner describe")
}
