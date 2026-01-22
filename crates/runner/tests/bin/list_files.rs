#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
name = "list_files"
edition = "2024"

[package.metadata.wasi-tool]
name = "list_files"
version = "1.0.0"
description = "Lists files in the current directory"

[package.metadata.wasi-tool.capabilities]
read = true
write = false
net = false

[dependencies]
serde_json = { workspace = true }
---

fn main() {
    let entries: Vec<String> = std::fs::read_dir(".")
        .map(|dir| {
            dir.filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();

    println!("{}", serde_json::to_string_pretty(&entries).unwrap());
}
