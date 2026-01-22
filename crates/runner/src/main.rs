use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use wasi_runner::{
    ensure_compiled, extract, extract_target_from_source, is_cached, list_tools, run_wasm,
    Capabilities,
};

#[derive(Parser)]
#[command(name = "wasi-runner")]
#[command(about = "WASI runner for OpenCode sandboxed tools")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a WASI tool
    Run {
        /// Path to the .rs tool file
        path: PathBuf,

        /// Allow read access to directory (can be repeated)
        #[arg(long = "allow-read", value_name = "PATH")]
        allow_read: Vec<PathBuf>,

        /// Allow write access to directory (can be repeated)
        #[arg(long = "allow-write", value_name = "PATH")]
        allow_write: Vec<PathBuf>,

        /// Allow network access
        #[arg(long = "allow-net")]
        allow_net: bool,

        /// Arguments to pass to the tool
        #[arg(last = true)]
        args: Vec<String>,
    },

    /// Get tool metadata as JSON
    Describe {
        /// Path to the .rs tool file
        path: PathBuf,
    },

    /// List available tools in a directory
    List {
        /// Directory containing .rs tool files
        path: PathBuf,
    },

    /// Check if a tool needs recompilation
    Check {
        /// Path to the .rs tool file
        path: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let project_root = std::env::current_dir()?;

    match cli.command {
        Commands::Run {
            path,
            allow_read,
            allow_write,
            allow_net,
            args,
        } => {
            let target = extract_target_from_source(&path)?;
            let wasm_path = ensure_compiled(&project_root, &path)?;

            let caps = Capabilities {
                read_dirs: allow_read,
                write_dirs: allow_write,
                allow_net,
            };

            let result = run_wasm(&wasm_path, &args, &caps, target)?;

            print!("{}", result.stdout);
            eprint!("{}", result.stderr);

            std::process::exit(result.exit_code);
        }

        Commands::Describe { path } => {
            let target = extract_target_from_source(&path)?;
            let wasm_path = ensure_compiled(&project_root, &path)?;
            let meta = extract(&wasm_path, target)?;
            println!("{}", serde_json::to_string_pretty(&meta)?);
        }

        Commands::List { path } => {
            let tools = list_tools(&path)?;
            for tool in tools {
                println!("{}", tool.display());
            }
        }

        Commands::Check { path } => {
            if let Some(cached) = is_cached(&project_root, &path)? {
                println!("cached: {}", cached.display());
                std::process::exit(0);
            } else {
                println!("needs compilation");
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
