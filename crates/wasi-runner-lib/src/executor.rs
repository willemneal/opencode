use anyhow::Result;
use std::path::{Path, PathBuf};
use wasi_tool_error::WasiTarget;
use wasmtime::{Engine, Store};
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

/// Run a WASM module/component with the appropriate WASI runtime
pub fn run_wasm(
    wasm_path: &Path,
    args: &[String],
    caps: &Capabilities,
    target: WasiTarget,
) -> Result<ExecutionResult> {
    match target {
        WasiTarget::Preview1 => run_preview1(wasm_path, args, caps),
        WasiTarget::Preview2 => run_preview2(wasm_path, args, caps),
    }
}

/// Run a WASI preview1 module (wasm32-wasip1)
fn run_preview1(wasm_path: &Path, args: &[String], caps: &Capabilities) -> Result<ExecutionResult> {
    use wasmtime::Module;
    use wasmtime_wasi::preview1::{self, WasiP1Ctx};

    struct WasiHostCtx {
        preview1: WasiP1Ctx,
        stdout: wasmtime_wasi::pipe::MemoryOutputPipe,
        stderr: wasmtime_wasi::pipe::MemoryOutputPipe,
    }

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

    let host_ctx = WasiHostCtx {
        preview1: wasi.build_p1(),
        stdout: stdout.clone(),
        stderr: stderr.clone(),
    };

    let mut store = Store::new(&engine, host_ctx);

    let mut linker: wasmtime::Linker<WasiHostCtx> = wasmtime::Linker::new(&engine);
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

/// Run a WASI preview2 component (wasm32-wasip2)
fn run_preview2(wasm_path: &Path, args: &[String], caps: &Capabilities) -> Result<ExecutionResult> {
    use wasmtime::component::{Component, Linker, ResourceTable};
    use wasmtime_wasi::bindings::sync::Command;
    use wasmtime_wasi::{WasiCtx, WasiView};
    use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};

    struct WasiHostCtx {
        ctx: WasiCtx,
        http: WasiHttpCtx,
        table: ResourceTable,
        stdout: wasmtime_wasi::pipe::MemoryOutputPipe,
        stderr: wasmtime_wasi::pipe::MemoryOutputPipe,
    }

    impl WasiView for WasiHostCtx {
        fn ctx(&mut self) -> &mut WasiCtx {
            &mut self.ctx
        }
        fn table(&mut self) -> &mut ResourceTable {
            &mut self.table
        }
    }

    impl WasiHttpView for WasiHostCtx {
        fn ctx(&mut self) -> &mut WasiHttpCtx {
            &mut self.http
        }
        fn table(&mut self) -> &mut ResourceTable {
            &mut self.table
        }
    }

    let engine = Engine::default();
    let component = Component::from_file(&engine, wasm_path)?;

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
        wasi.allow_ip_name_lookup(true);
    }

    let host_ctx = WasiHostCtx {
        ctx: wasi.build(),
        http: WasiHttpCtx::new(),
        table: ResourceTable::new(),
        stdout: stdout.clone(),
        stderr: stderr.clone(),
    };

    let mut store = Store::new(&engine, host_ctx);

    let mut linker: Linker<WasiHostCtx> = Linker::new(&engine);

    // Add full WASI interfaces to linker
    wasmtime_wasi::add_to_linker_sync(&mut linker)?;

    // Add HTTP interfaces for networking support
    wasmtime_wasi_http::add_only_http_to_linker_sync(&mut linker)?;

    let command = Command::instantiate(&mut store, &component, &linker)?;
    let result = command.wasi_cli_run().call_run(&mut store);

    // Preview2 exit codes are limited to 0/1 via the wasi:cli/run interface
    let exit_code = match result {
        Ok(Ok(())) => 0,
        Ok(Err(())) => 1,
        Err(e) => {
            eprintln!("WASM trap: {}", e);
            1
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
