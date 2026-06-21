//! `prest` library: Static API generator core logic
//!
//! This crate provides serializers, delivery layouts, and orchestration logic
//! for compiling raw JSON datasets into structured static API endpoints.

use thiserror::Error;
use std::path::{Path, PathBuf};

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
}

pub mod serializers;
pub mod orchestrator;

pub use serializers::{Serializer, JSONSerializer, TypescriptSerializer, SqliteSerializer};
pub use orchestrator::{run, Config, SerializerConfig};

/// Trait for physical data serialization
pub trait Layout {
    /// Determines the physical path based on endpoint, file extension, and collection status
    ///
    /// # Examples
    ///
    /// ```
    /// use prest::{IndexLayout, Layout};
    /// use std::path::Path;
    ///
    /// let layout = IndexLayout;
    /// let path = layout.determine_path("api/test", "json", false);
    /// assert_eq!(path, std::path::Path::new("api/test/index.json"));
    /// ```
    fn determine_path(&self, endpoint: &str, file_ext: &str, is_coll: bool) -> PathBuf;
}

/// Layout that places files in `index.[ext]`
pub struct IndexLayout;
impl Layout for IndexLayout {
    fn determine_path(&self, endpoint: &str, ext: &str, _: bool) -> PathBuf {
        Path::new(endpoint).join(format!("index.{}", ext))
    }
}

/// Layout that appends the extension directly
pub struct ExtensionLayout;
impl Layout for ExtensionLayout {
    fn determine_path(&self, endpoint: &str, ext: &str, _: bool) -> PathBuf {
        PathBuf::from(format!("{}.{}", endpoint, ext))
    }
}

/// Layout that avoids extensions where possible (supports smart fallback)
pub struct FileLayout;
impl FileLayout {
    fn is_coll_path(&self, endpoint: &str, is_coll: bool) -> bool {
        is_coll && !endpoint.is_empty()
    }
}
impl Layout for FileLayout {
    fn determine_path(&self, endpoint: &str, ext: &str, is_coll: bool) -> PathBuf {
        if self.is_coll_path(endpoint, is_coll) {
            Path::new(endpoint).join(format!("index.{}", ext))
        } else {
            PathBuf::from(endpoint)
        }
    }
}
