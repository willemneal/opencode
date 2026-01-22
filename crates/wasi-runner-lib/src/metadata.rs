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

/// Extract metadata from the cargo frontmatter of a source file
/// Looks for [package.metadata.wasi-tool] section
pub fn extract_from_source(source: &Path) -> Result<ToolMetadata> {
    let content = std::fs::read_to_string(source).context("Failed to read source file")?;
    extract_from_frontmatter(&content)
}

/// Parse metadata from cargo frontmatter content
fn extract_from_frontmatter(source: &str) -> Result<ToolMetadata> {
    let lines: Vec<&str> = source.lines().collect();

    // Find the start marker (---cargo)
    let start = lines
        .iter()
        .position(|line| line.trim() == "---cargo")
        .context("No ---cargo frontmatter found")?;

    // Find the end marker (---)
    let end = lines
        .iter()
        .skip(start + 1)
        .position(|line| line.trim() == "---")
        .context("No closing --- for frontmatter")?
        + start
        + 1;

    // Extract manifest (between markers)
    let manifest = lines[start + 1..end].join("\n");

    // Parse as TOML
    let parsed: toml::Value = toml::from_str(&manifest).context("Failed to parse TOML manifest")?;

    // Extract [package.metadata.wasi-tool]
    let metadata = parsed
        .get("package")
        .and_then(|p| p.get("metadata"))
        .and_then(|m| m.get("wasi-tool"))
        .context("No [package.metadata.wasi-tool] section found")?;

    let name = metadata
        .get("name")
        .and_then(|v| v.as_str())
        .context("Missing 'name' in wasi-tool metadata")?
        .to_string();

    let version = metadata
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0")
        .to_string();

    let description = metadata
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Parse capabilities
    let caps = metadata.get("capabilities");
    let capabilities = wasi_tool_error::Capabilities {
        read: caps
            .and_then(|c| c.get("read"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        write: caps
            .and_then(|c| c.get("write"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        net: caps
            .and_then(|c| c.get("net"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    };

    // Parse args if present
    let args = metadata
        .get("args")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|arg| {
                    Some(wasi_tool_error::ArgSpec {
                        name: arg.get("name")?.as_str()?.to_string(),
                        arg_type: arg.get("type")?.as_str()?.to_string(),
                        description: arg
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        required: arg
                            .get("required")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        default: arg
                            .get("default")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // Parse errors if present
    let errors = metadata
        .get("errors")
        .and_then(|e| e.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|err| {
                    Some(wasi_tool_error::ErrorSpec {
                        code: err.get("code")?.as_integer()? as i32,
                        message: err.get("message")?.as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(ToolMetadata {
        name,
        version,
        description,
        args,
        errors,
        capabilities,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_from_frontmatter() {
        let source = r#"#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
name = "test_tool"
edition = "2024"

[package.metadata.wasi-tool]
name = "test_tool"
version = "1.0.0"
description = "A test tool"

[package.metadata.wasi-tool.capabilities]
read = true
write = false
net = false

[[package.metadata.wasi-tool.args]]
name = "input"
type = "string"
description = "Input file"
required = true

[[package.metadata.wasi-tool.errors]]
code = 100
message = "Custom error"

[dependencies]
serde_json = "1"
---

fn main() {}
"#;

        let metadata = extract_from_frontmatter(source).unwrap();
        assert_eq!(metadata.name, "test_tool");
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.description, "A test tool");
        assert!(metadata.capabilities.read);
        assert!(!metadata.capabilities.write);
        assert!(!metadata.capabilities.net);
        assert_eq!(metadata.args.len(), 1);
        assert_eq!(metadata.args[0].name, "input");
        assert_eq!(metadata.errors.len(), 1);
        assert_eq!(metadata.errors[0].code, 100);
    }
}
