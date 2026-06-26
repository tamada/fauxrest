//! `prest` library: Static API generator core logic
//!
//! This crate provides serializers, delivery layouts, and orchestration logic
//! for compiling raw JSON datasets into structured static API endpoints.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

/// Error type for prest
#[derive(Error, Debug)]
pub enum Error {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization error
    #[error("JSON serialization error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    /// SQLite error
    #[error("SQLite error: {0}")]
    Rusqlite(#[from] rusqlite::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Unknown serializer error
    #[error("{0}: Unknown serializer")]
    UnknownSerializer(String),
}

pub mod config;
pub mod serializers;
pub mod orchestrator;

pub use config::{Config, Layout, SerializerConfig};
pub use serializers::{Serializer, JSONSerializer, TypescriptSerializer, SqliteSerializer};
pub use orchestrator::{run};
