//! `fauxrest` library: Static API generator core logic
//!
//! This crate provides serializers, delivery layouts, and orchestration logic
//! for compiling raw JSON datasets into structured static API endpoints.

use std::{
    collections::HashSet,
    sync::{Mutex, OnceLock},
};

use serde_json::Value;
use thiserror::Error;

/// Convenience alias for `Result<T, Error>` used throughout this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for fauxrest
#[derive(Error, Debug)]
pub enum Error {
    /// A value could not be converted from the first type name to the second.
    #[error("{0}: failed to convert to {1}")]
    Cast(String, String),

    /// Command-line argument parsing failed (propagated from `clap`).
    #[error("{0}")]
    Clap(#[from] clap::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Destination directory already contains files and overwrite is disabled
    #[error("{0}: dest is not empty, use --overwrite to overwrite existing files")]
    DestNotEmpty(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization error
    #[error("JSON serialization error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    /// SQLite error
    #[error("SQLite error: {0}")]
    Rusqlite(#[from] rusqlite::Error),

    /// Unknown serializer error
    #[error("{0}: Unknown serializer")]
    UnknownSerializer(String),
}

/// Configuration parsing and validation (`_config.json` schema).
pub mod config;
/// Internal serializer/layout wiring shared by the orchestrator.
mod context;
/// `$filter` directive types and evaluation logic.
pub mod filter;
/// Compilation orchestrator: reads raw JSON data and writes static endpoints.
pub mod orchestrator;
/// Output serializers (JSON, TypeScript, SQLite).
pub mod serializers;
/// Internal `$static` copy support: copies allowed non-JSON static files
/// from the data directory into each serializer destination.
mod static_files;

pub use config::{Config, Layout, SerializerConfig, StaticConfig, StaticSpec};
pub use orchestrator::run;
pub use serializers::{JSONSerializer, Serializer, SqliteSerializer, TypescriptSerializer};

use crate::config::FilterOp;

/// Process-wide set of already-emitted `$filter` type-mismatch warning keys,
/// used to deduplicate repeated warnings printed to stderr.
static TYPE_MISMATCH_WARNINGS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

/// Prints a one-time warning to stderr when a `$filter` comparison mixes
/// incompatible JSON value types (e.g. comparing a string field against a
/// numeric literal).
///
/// The warning is deduplicated per `(field, op, lhs kind, rhs kind)` tuple
/// via [`TYPE_MISMATCH_WARNINGS`], so the same mismatch is only reported once
/// per process even if it occurs for many items.
pub(crate) fn emit_type_mismatch_warning(op: &FilterOp, field: &str, lhs: &Value, rhs: &Value) {
    let lhs_kind = value_kind(lhs);
    let rhs_kind = value_kind(rhs);
    let op_str = op.to_string();
    let key = format!("{}|{}|{}|{}", field, op_str, lhs_kind, rhs_kind);
    let warnings = TYPE_MISMATCH_WARNINGS.get_or_init(|| Mutex::new(HashSet::new()));
    let mut guard = match warnings.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    if guard.insert(key) {
        eprintln!(
            "warning: $filter type mismatch for field '{}' with op '{}': lhs is {}, rhs is {}",
            field, op_str, lhs_kind, rhs_kind
        );
    }
}

/// Returns a short, human-readable name for the kind of a `serde_json::Value`
/// (e.g. `"string"`, `"number"`), used for diagnostics and type-compatibility checks.
pub(crate) fn value_kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
