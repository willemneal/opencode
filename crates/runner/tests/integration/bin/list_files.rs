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
---

fn main() {
    print!("[");
    let mut first = true;
    if let Ok(dir) = std::fs::read_dir(".") {
        for entry in dir.flatten() {
            if !first {
                print!(",");
            }
            first = false;
            print!("\"{}\"", entry.file_name().to_string_lossy());
        }
    }
    println!("]");
}
