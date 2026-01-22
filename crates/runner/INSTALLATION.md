# WASI Runner Installation

The WASI runner compiles and executes sandboxed Rust tools using WebAssembly.

## Prerequisites

### 1. Rust Nightly Toolchain

```bash
rustup install nightly
```

### 2. WASI Target

```bash
rustup target add wasm32-wasip1 --toolchain nightly
```

## Building the Runner

From the repository root:

```bash
cd crates/runner
cargo build --release
```

The binary will be available at `crates/runner/target/release/wasi-runner`.

## Usage

### Run a WASI tool

```bash
wasi-runner run crates/bin/file_search.rs \
  --allow-read=/path/to/project \
  -- "*.ts" "./src"
```

### Get tool metadata (JSON)

```bash
wasi-runner describe crates/bin/file_search.rs
```

### List available tools

```bash
wasi-runner list crates/bin/
```

### Check if recompilation needed

```bash
wasi-runner check crates/bin/file_search.rs
```

## Capabilities

Tools run in a sandboxed WASI environment. You must explicitly grant capabilities:

| Flag                 | Description                     |
| -------------------- | ------------------------------- |
| `--allow-read=PATH`  | Grant read access to directory  |
| `--allow-write=PATH` | Grant write access to directory |
| `--allow-net`        | Grant network access            |

Multiple paths can be specified by repeating the flag.

## Caching

Compiled WASM files are cached in `.opencode/cache/wasi/` using git blob hashes.
The cache is automatically invalidated when the source file content changes.

## Error Codes

| Code | Meaning              |
| ---- | -------------------- |
| 0    | Success              |
| 1    | Invalid arguments    |
| 2    | File not found       |
| 3    | Permission denied    |
| 4    | Network error        |
| 5    | Parse error          |
| 6    | Timeout              |
| 99   | Internal error       |
| 100+ | Tool-specific errors |

## Creating a New Tool

Tools are single-file Rust scripts using cargo script format (RFC 3503). Example:

```rust
#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
name = "my_tool"
edition = "2024"

[dependencies]
serde_json = "1"
wasi-tool-error = { path = "../wasi-tool-error" }
---

use std::{env, process};
use wasi_tool_error::{codes, ToolMetadata, ArgSpec, ErrorSpec, Capabilities};

fn metadata() -> ToolMetadata {
    ToolMetadata {
        name: "my_tool".to_string(),
        version: "1.0.0".to_string(),
        description: "Description of what your tool does".to_string(),
        args: vec![
            ArgSpec {
                name: "input".to_string(),
                arg_type: "string".to_string(),
                description: "Input file path".to_string(),
                required: true,
                default: None,
            },
        ],
        errors: vec![
            ErrorSpec { code: 100, message: "Tool-specific error".to_string() },
        ],
        capabilities: Capabilities { read: true, write: false, net: false },
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.get(1).map(|s| s.as_str()) == Some("--describe") {
        wasi_tool_error::print_metadata(&metadata());
        return;
    }

    // Your tool logic here
    println!("Hello from my_tool!");
}
```

Place your tool in `crates/bin/` or `.opencode/wasi-tools/` to be auto-discovered.
