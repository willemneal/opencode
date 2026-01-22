use anyhow::Result;
use std::path::{Path, PathBuf};
use wasmtime::*;
use wasmtime_wasi::preview1::{self, WasiP1Ctx};
use wasmtime_wasi::{DirPerms, FilePerms, I32Exit, WasiCtxBuilder};

pub struct Capabilities {
    pub read_dirs: Vec<PathBuf>,
    pub write_dirs: Vec<PathBuf>,
    pub allow_net: bool,
}

pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

struct WasiHostCtx {
    preview1: WasiP1Ctx,
    stdout: wasmtime_wasi::pipe::MemoryOutputPipe,
    stderr: wasmtime_wasi::pipe::MemoryOutputPipe,
}

pub fn run_wasm(wasm_path: &Path, args: &[String], caps: &Capabilities) -> Result<ExecutionResult> {
    let engine = Engine::default();
    let module = Module::from_file(&engine, wasm_path)?;

    let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(1024 * 1024);
    let stderr = wasmtime_wasi::pipe::MemoryOutputPipe::new(1024 * 1024);

    let mut wasi = WasiCtxBuilder::new();

    // Set program name and arguments
    let mut full_args = vec![wasm_path.to_string_lossy().to_string()];
    full_args.extend(args.iter().cloned());
    wasi.args(&full_args);

    // Set stdout/stderr
    wasi.stdout(stdout.clone());
    wasi.stderr(stderr.clone());

    // Configure read-only directories
    for dir in &caps.read_dirs {
        if dir.exists() {
            let dir_str = dir.to_string_lossy().to_string();
            wasi.preopened_dir(dir, &dir_str, DirPerms::READ, FilePerms::READ)?;
        }
    }

    // Configure read-write directories
    for dir in &caps.write_dirs {
        if dir.exists() {
            let dir_str = dir.to_string_lossy().to_string();
            wasi.preopened_dir(dir, &dir_str, DirPerms::all(), FilePerms::all())?;
        }
    }

    // Network access
    if caps.allow_net {
        wasi.inherit_network();
    }

    let preview1 = wasi.build_p1();

    let host_ctx = WasiHostCtx {
        preview1,
        stdout: stdout.clone(),
        stderr: stderr.clone(),
    };

    let mut store = Store::new(&engine, host_ctx);

    let mut linker: Linker<WasiHostCtx> = Linker::new(&engine);
    preview1::add_to_linker_sync(&mut linker, |ctx: &mut WasiHostCtx| &mut ctx.preview1)?;

    let instance = linker.instantiate(&mut store, &module)?;

    // Call _start (WASI entry point)
    let start = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
    let result = start.call(&mut store, ());

    let exit_code = match result {
        Ok(()) => 0,
        Err(e) => {
            // Try to extract exit code from WASI trap
            if let Some(exit) = e.downcast_ref::<I32Exit>() {
                exit.0
            } else {
                eprintln!("WASM trap: {}", e);
                1
            }
        }
    };

    let stdout_bytes = store.data().stdout.contents();
    let stderr_bytes = store.data().stderr.contents();

    Ok(ExecutionResult {
        exit_code,
        stdout: String::from_utf8_lossy(&stdout_bytes).to_string(),
        stderr: String::from_utf8_lossy(&stderr_bytes).to_string(),
    })
}
