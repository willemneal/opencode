use crate::executor::{self, Capabilities};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use wasi_tool_error::ToolMetadata;

/// Extract metadata from a compiled WASM tool by running --describe
pub fn extract(wasm_path: &Path) -> Result<ToolMetadata> {
    let caps = Capabilities {
        read_dirs: vec![],
        write_dirs: vec![],
        allow_net: false,
    };

    let result = executor::run_wasm(wasm_path, &["--describe".to_string()], &caps)?;

    if result.exit_code != 0 {
        anyhow::bail!(
            "Tool --describe failed with code {}: {}",
            result.exit_code,
            result.stderr
        );
    }

    serde_json::from_str(&result.stdout).context("Failed to parse tool metadata")
}

/// List all .rs tool files in a directory
pub fn list_tools(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut tools = Vec::new();

    if !dir.exists() {
        return Ok(tools);
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "rs").unwrap_or(false) {
            tools.push(path);
        }
    }

    tools.sort();
    Ok(tools)
}
