#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
name = "file_search"
edition = "2024"

[dependencies]
serde_json = { workspace = true }
glob = { workspace = true }
clap = { workspace = true }
wasi-tool-error = { workspace = true }
---

use clap::Parser;
use std::process;
use wasi_tool_error::{ArgSpec, Capabilities, ErrorSpec, ToolMetadata};

#[derive(Parser)]
#[command(name = "file_search")]
#[command(about = "Searches for files matching a glob pattern in directories")]
struct Args {
    /// Print tool metadata as JSON
    #[arg(long)]
    describe: bool,

    /// Glob pattern to match files (e.g., '*.rs', '**/*.ts')
    #[arg(long)]
    pattern: Option<String>,

    /// Directory to search in
    #[arg(long, default_value = ".")]
    directory: String,
}

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
    let args = Args::parse();

    if args.describe {
        wasi_tool_error::print_metadata(&metadata());
        return;
    }

    let pattern = match args.pattern {
        Some(p) => p,
        None => {
            eprintln!("Error: --pattern is required");
            process::exit(1);
        }
    };

    if !std::path::Path::new(&args.directory).exists() {
        eprintln!("Error: directory '{}' does not exist", args.directory);
        process::exit(101);
    }

    let full_pattern = format!("{}/{}", args.directory, pattern);

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

    println!("{}", serde_json::to_string_pretty(&matches).unwrap());
}
