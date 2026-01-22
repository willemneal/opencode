#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
name = "github_pr_search"
edition = "2024"

[package.metadata.wasi-tool]
wasi_target = "preview2"

[package.metadata.wasi-tool.capabilities]
read = false
write = false
net = true

[dependencies]
serde_json = { workspace = true }
serde = { version = "1", features = ["derive"] }
clap = { workspace = true }
wasi-tool-error = { workspace = true }
wasi-http-client = { version = "0.2", features = ["json"] }
---

use clap::Parser;
use serde::Deserialize;
use std::process;
use wasi_http_client::Client;
use wasi_tool_error::{ArgSpec, Capabilities, ErrorSpec, ToolMetadata, WasiTarget};

#[derive(Parser)]
#[command(name = "github_pr_search")]
#[command(about = "Searches GitHub for related PRs")]
struct Args {
    /// Print tool metadata as JSON
    #[arg(long)]
    describe: bool,

    /// Search query for PR titles and descriptions
    #[arg(long)]
    query: Option<String>,

    /// GitHub repository owner
    #[arg(long, default_value = "anomalyco")]
    owner: String,

    /// GitHub repository name
    #[arg(long, default_value = "opencode")]
    repo: String,

    /// Maximum number of results to return
    #[arg(long, default_value = "10")]
    limit: u32,

    /// PR state filter (open, closed, all)
    #[arg(long, default_value = "open")]
    state: String,
}

#[derive(Deserialize)]
struct SearchResponse {
    total_count: u32,
    items: Vec<PullRequest>,
}

#[derive(Deserialize)]
struct PullRequest {
    title: String,
    html_url: String,
    number: u32,
    state: String,
    user: User,
}

#[derive(Deserialize)]
struct User {
    login: String,
}

#[derive(serde::Serialize)]
struct Output {
    total: u32,
    showing: usize,
    prs: Vec<PrOutput>,
}

#[derive(serde::Serialize)]
struct PrOutput {
    number: u32,
    title: String,
    url: String,
    state: String,
    author: String,
}

fn metadata() -> ToolMetadata {
    ToolMetadata {
        name: "github_pr_search".to_string(),
        version: "1.0.0".to_string(),
        description: "Searches GitHub for related PRs".to_string(),
        args: vec![
            ArgSpec {
                name: "query".to_string(),
                arg_type: "string".to_string(),
                description: "Search query for PR titles and descriptions".to_string(),
                required: true,
                default: None,
            },
            ArgSpec {
                name: "owner".to_string(),
                arg_type: "string".to_string(),
                description: "GitHub repository owner".to_string(),
                required: false,
                default: Some("anomalyco".to_string()),
            },
            ArgSpec {
                name: "repo".to_string(),
                arg_type: "string".to_string(),
                description: "GitHub repository name".to_string(),
                required: false,
                default: Some("opencode".to_string()),
            },
            ArgSpec {
                name: "limit".to_string(),
                arg_type: "number".to_string(),
                description: "Maximum number of results to return".to_string(),
                required: false,
                default: Some("10".to_string()),
            },
            ArgSpec {
                name: "state".to_string(),
                arg_type: "string".to_string(),
                description: "PR state filter (open, closed, all)".to_string(),
                required: false,
                default: Some("open".to_string()),
            },
        ],
        errors: vec![
            ErrorSpec {
                code: 100,
                message: "GitHub API request failed".to_string(),
            },
            ErrorSpec {
                code: 101,
                message: "Invalid response from GitHub".to_string(),
            },
        ],
        capabilities: Capabilities {
            read: false,
            write: false,
            net: true,
        },
        wasi_target: WasiTarget::Preview2,
    }
}

fn main() {
    let args = Args::parse();

    if args.describe {
        wasi_tool_error::print_metadata(&metadata());
        return;
    }

    let query = match args.query {
        Some(q) => q,
        None => {
            eprintln!("Error: --query is required");
            process::exit(1);
        }
    };

    let token = std::env::var("GITHUB_TOKEN").ok();

    let state = match args.state.as_str() {
        "open" | "closed" | "all" => &args.state,
        _ => {
            eprintln!("Error: --state must be one of: open, closed, all");
            process::exit(1);
        }
    };

    let search = format!(
        "{} repo:{}/{} type:pr{}",
        query,
        args.owner,
        args.repo,
        if state != "all" {
            format!(" state:{}", state)
        } else {
            String::new()
        }
    );

    let url = format!(
        "https://api.github.com/search/issues?q={}&per_page={}&sort=updated&order=desc",
        urlencoding(&search),
        args.limit
    );

    let client = Client::new();
    let mut request = client
        .get(&url)
        .header("Accept", b"application/vnd.github+json".to_vec())
        .header("User-Agent", b"github_pr_search/1.0".to_vec());

    if let Some(t) = &token {
        request = request.header("Authorization", format!("Bearer {}", t).into_bytes());
    }

    let response = match request.send() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: GitHub API request failed: {}", e);
            process::exit(100);
        }
    };

    let result: SearchResponse = match response.json() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: Invalid response from GitHub: {}", e);
            process::exit(101);
        }
    };

    let output = Output {
        total: result.total_count,
        showing: result.items.len(),
        prs: result
            .items
            .into_iter()
            .map(|pr| PrOutput {
                number: pr.number,
                title: pr.title,
                url: pr.html_url,
                state: pr.state,
                author: pr.user.login,
            })
            .collect(),
    };

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

fn urlencoding(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(c),
            ' ' => result.push_str("%20"),
            ':' => result.push_str("%3A"),
            '/' => result.push_str("%2F"),
            _ => {
                for b in c.to_string().as_bytes() {
                    result.push_str(&format!("%{:02X}", b));
                }
            }
        }
    }
    result
}
