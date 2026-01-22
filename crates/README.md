# OpenCode WASI Sandboxed Tools

This directory contains Rust crates for running sandboxed tools using WASI (WebAssembly System Interface).

## Overview

The WASI tool system allows agents to use sandboxed tools with fine-grained capability restrictions:

- File system access (read/write to specific directories)
- Network access (can be enabled/disabled)
- No access to host environment by default

This provides a secure way for agents to execute tools without full system access.

## Directory Structure

```
crates/
├── bin/                    # Single-file Rust tools (cargo script format)
│   └── file_search.rs      # Example: sandboxed file search tool
├── runner/                 # WASI runner (compiles & executes tools)
│   ├── Cargo.toml
│   ├── INSTALLATION.md     # Setup instructions
│   └── src/
│       ├── main.rs         # CLI entry point
│       ├── cache.rs        # Git blob hash caching
│       ├── compiler.rs     # Cargo script compilation
│       ├── executor.rs     # Wasmtime execution
│       └── metadata.rs     # Tool metadata extraction
└── wasi-tool-error/        # Shared error types
    ├── Cargo.toml
    └── src/lib.rs
```

## Quick Start

See [runner/INSTALLATION.md](runner/INSTALLATION.md) for detailed setup instructions.

### Prerequisites

```bash
# Install Rust nightly
rustup install nightly

# Add WASI target
rustup target add wasm32-wasip1 --toolchain nightly

# Build the runner
cd crates/runner
cargo build --release
```

### Usage

```bash
# Run a tool with read access to current directory
./target/release/wasi-runner run crates/bin/file_search.rs \
  --allow-read=. \
  -- "*.rs" "."

# Get tool metadata
./target/release/wasi-runner describe crates/bin/file_search.rs
```

## Creating Tools

Tools are single-file Rust scripts using the cargo script format (RFC 3503).
They must implement a `--describe` flag that outputs JSON metadata.

See [bin/file_search.rs](bin/file_search.rs) for an example.

## Integration with OpenCode

Tools placed in `.opencode/wasi-tools/` are automatically discovered and available to agents.
Configure capabilities in `opencode.json`:

```json
{
  "wasi": {
    "runner": "crates/runner/target/release/wasi-runner"
  }
}
```
