#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
name = "file_search"
edition = "2024"

[dependencies]
serde_json = "1"
glob = "0.3"
wasi-tool-error = { path = "../wasi-tool-error" }
---

use std::{env, process};
use wasi_tool_error::{codes, ArgSpec, Capabilities, ErrorSpec, ToolMetadata};

fn metadata() -> ToolMetadata {
    ToolMetadata {
        name: "file_search".to_string(),
        version: "1.0.0".to_string(),
        description: "Searches for files matching a glob pattern in directories".to_string(),
        args: vec![
            ArgSpec {
                name: "pattern".to_string(),
                arg_type: "string".to_string(),
                description: "Glob pattern to match files (e.g., '*.rs', '**/*.ts')".to_string(),
                required: true,
                default: None,
            },
            ArgSpec {
                name: "directory".to_string(),
                arg_type: "string".to_string(),
                description: "Directory to search in".to_string(),
                required: false,
                default: Some(".".to_string()),
            },
        ],
        errors: vec![
            ErrorSpec {
                code: 100,
                message: "Invalid glob pattern".to_string(),
            },
            ErrorSpec {
                code: 101,
                message: "Directory does not exist".to_string(),
            },
        ],
        capabilities: Capabilities {
            read: true,
            write: false,
            net: false,
        },
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Handle --describe for metadata extraction
    if args.get(1).map(|s| s.as_str()) == Some("--describe") {
        wasi_tool_error::print_metadata(&metadata());
        return;
    }

    // Parse arguments
    let pattern = match args.get(1) {
        Some(p) => p,
        None => {
            eprintln!("Error: pattern argument required");
            eprintln!("Usage: file_search <pattern> [directory]");
            process::exit(codes::INVALID_ARGS);
        }
    };

    let directory = args.get(2).map(|s| s.as_str()).unwrap_or(".");

    // Validate directory exists
    if !std::path::Path::new(directory).exists() {
        eprintln!("Error: directory '{}' does not exist", directory);
        process::exit(101);
    }

    // Build full pattern
    let full_pattern = format!("{}/{}", directory, pattern);

    // Search files
    let matches: Vec<String> = match glob::glob(&full_pattern) {
        Ok(paths) => paths
            .filter_map(|p| p.ok())
            .filter(|p| p.is_file())
            .map(|p| p.display().to_string())
            .collect(),
        Err(e) => {
            eprintln!("Error: invalid glob pattern: {}", e);
            process::exit(100);
        }
    };

    // Output results as JSON
    println!("{}", serde_json::to_string_pretty(&matches).unwrap());
}
