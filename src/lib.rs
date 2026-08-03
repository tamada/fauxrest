//! `fauxrest` library: Static API generator core logic
//!
//! This crate provides serializers, delivery layouts, and orchestration logic
//! for compiling raw JSON datasets into structured static API endpoints.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use regex::Regex;
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

    /// A `$filter` condition met operand kinds it cannot evaluate, either
    /// because the record and the condition hold different JSON kinds or
    /// because the operator does not support the kind they share.
    #[error("$filter type error: {0}")]
    FilterType(String),

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

/// Process-wide cache of compiled `$filter`/`$derive` patterns, keyed by the
/// pattern string.
static REGEX_CACHE: OnceLock<Mutex<HashMap<String, Arc<Regex>>>> = OnceLock::new();

/// Returns the compiled form of `pattern`, compiling it on first use and
/// sharing it afterwards.
///
/// `$filter` and `$derive` both evaluate their patterns once per record, and
/// compiling a regex costs far more than matching with it. Patterns come from
/// the configuration, so the same handful is reused for every item — caching
/// removes work that otherwise scales with
/// (items x conditions x serializers). Config validation compiles through
/// this cache too, so a validated pattern is already warm by the time the
/// orchestrator reaches it.
///
/// The `regex::Error` is returned as-is so each caller can name the directive
/// it came from. Invalid patterns are not cached; validation rejects them
/// before evaluation anyway.
pub(crate) fn compile_regex(pattern: &str) -> std::result::Result<Arc<Regex>, regex::Error> {
    let cache = REGEX_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock()
        && let Some(re) = guard.get(pattern)
    {
        return Ok(Arc::clone(re));
    }
    let re = Arc::new(Regex::new(pattern)?);
    if let Ok(mut guard) = cache.lock() {
        guard.insert(pattern.to_string(), Arc::clone(&re));
    }
    Ok(re)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The same pattern must be compiled once and shared afterwards — that is
    /// the whole point of the cache.
    #[test]
    fn test_identical_patterns_share_one_compiled_regex() {
        let first = compile_regex("^ab+c$").expect("valid pattern");
        let second = compile_regex("^ab+c$").expect("valid pattern");
        assert!(Arc::ptr_eq(&first, &second));
    }

    /// Distinct patterns must not collide in the cache.
    #[test]
    fn test_distinct_patterns_are_compiled_separately() {
        let a = compile_regex("^shared-cache-a$").expect("valid pattern");
        let b = compile_regex("^shared-cache-b$").expect("valid pattern");
        assert!(!Arc::ptr_eq(&a, &b));
        assert!(a.is_match("shared-cache-a"));
        assert!(b.is_match("shared-cache-b"));
    }

    /// An invalid pattern surfaces the `regex` error so callers can name the
    /// directive it came from.
    #[test]
    fn test_invalid_pattern_returns_the_regex_error() {
        assert!(compile_regex("([unclosed").is_err());
    }
}
