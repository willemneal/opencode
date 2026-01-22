use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Standard exit codes (0-99 reserved for common errors).
pub mod codes {
    pub const SUCCESS: i32 = 0;
    pub const INVALID_ARGS: i32 = 1;
    pub const FILE_NOT_FOUND: i32 = 2;
    pub const PERMISSION_DENIED: i32 = 3;
    pub const NETWORK_ERROR: i32 = 4;
    pub const PARSE_ERROR: i32 = 5;
    pub const TIMEOUT: i32 = 6;
    pub const INTERNAL_ERROR: i32 = 99;
    /// Tool-specific errors start at 100.
    pub const TOOL_SPECIFIC_START: i32 = 100;
}

/// Base errors that all tools can use or extend with #[error(transparent)].
#[derive(Error, Debug, Serialize, Deserialize)]
pub enum BaseError {
    #[error("Invalid arguments: {0}")]
    InvalidArgs(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Timeout after {0}ms")]
    Timeout(u64),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl BaseError {
    #[must_use]
    pub fn code(&self) -> i32 {
        match self {
            Self::InvalidArgs(_) => codes::INVALID_ARGS,
            Self::FileNotFound(_) => codes::FILE_NOT_FOUND,
            Self::PermissionDenied(_) => codes::PERMISSION_DENIED,
            Self::NetworkError(_) => codes::NETWORK_ERROR,
            Self::ParseError(_) => codes::PARSE_ERROR,
            Self::Timeout(_) => codes::TIMEOUT,
            Self::Internal(_) => codes::INTERNAL_ERROR,
        }
    }
}

/// WASI target for compilation and execution.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WasiTarget {
    /// WASI preview1 - simpler, full exit code support, no networking.
    #[default]
    Preview1,
    /// WASI preview2 - networking support via wasi-http, exit codes limited to 0/1.
    Preview2,
}

/// Tool metadata embedded in binaries, returned via --describe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub args: Vec<ArgSpec>,
    pub errors: Vec<ErrorSpec>,
    pub capabilities: Capabilities,
    /// WASI target (preview1 or preview2), required to be preview2 if net capability is true.
    #[serde(default)]
    pub wasi_target: WasiTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgSpec {
    pub name: String,
    #[serde(rename = "type")]
    pub arg_type: String, // "string", "number", "boolean", "array"
    pub description: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorSpec {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Capabilities {
    #[serde(default)]
    pub read: bool,
    #[serde(default)]
    pub write: bool,
    #[serde(default)]
    pub net: bool,
}

/// Helper to print metadata JSON for --describe flag.
///
/// # Panics
/// Panics if metadata cannot be serialized to JSON (should never happen with valid metadata).
pub fn print_metadata(metadata: &ToolMetadata) {
    println!("{}", serde_json::to_string(metadata).unwrap());
}
