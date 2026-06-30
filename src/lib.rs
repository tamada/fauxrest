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

pub type Result<T> = std::result::Result<T, Error>;

/// Error type for fauxrest
#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}: failed to convert to {1}")]
    Cast(String, String),

    #[error("{0}")]
    Clap(#[from] clap::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

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

pub mod config;
mod context;
pub mod filter;
pub mod orchestrator;
pub mod serializers;

pub use config::{Config, Layout, SerializerConfig};
pub use orchestrator::run;
pub use serializers::{JSONSerializer, Serializer, SqliteSerializer, TypescriptSerializer};

use crate::config::FilterOp;

static TYPE_MISMATCH_WARNINGS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

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
