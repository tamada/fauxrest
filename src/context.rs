//! Wiring between a [`SerializerConfig`] and the concrete [`Serializer`] and
//! layout implementations it selects.
//!
//! [`SerializerContext`] bundles a resolved serializer, layout strategy, and
//! destination directory so the orchestrator can serialize data and compute
//! output paths without re-resolving the configuration on every call.

use std::path::PathBuf;

use serde_json::Value;

use crate::orchestrator::{ExtensionLayout, FileLayout, IndexLayout, LayoutTrait};
use crate::serializers::{JSONSerializer, SqliteSerializer, TypescriptSerializer};
use crate::{Error, Layout, Result, Serializer, SerializerConfig};

/// Resolved runtime context for one `$config` serializer entry: the
/// concrete [`Serializer`] and [`LayoutTrait`] implementations plus the
/// destination directory to write into.
pub struct SerializerContext {
    /// The serializer used to encode data into output bytes.
    pub serializer: Box<dyn Serializer>,
    /// The layout strategy used to compute output file paths.
    pub layout: Box<dyn LayoutTrait>,
    /// The destination directory that output paths are relative to.
    pub dest: PathBuf,
}

impl SerializerContext {
    /// Serializes `data` using this context's [`Serializer`].
    pub fn serialize(&self, data: &Value) -> Result<Vec<u8>> {
        self.serializer.serialize(data)
    }

    /// Computes the full output file path (including `dest`) for the
    /// endpoint named `name`, stripping a trailing `.json` suffix if
    /// present and applying the configured layout.
    pub fn full_path(&self, name: &str, is_coll: bool) -> PathBuf {
        let endpoint = name.strip_suffix(".json").unwrap_or(name);
        let path = self.determine_path(endpoint, is_coll);
        self.dest.join(path)
    }

    /// Computes the output path for `endpoint` relative to `dest`, using
    /// this context's layout and the serializer's file extension.
    /// `is_coll` indicates whether the data being written is a collection
    /// (array), which affects layouts like [`FileLayout`].
    pub fn determine_path(&self, endpoint: &str, is_coll: bool) -> PathBuf {
        self.layout
            .determine_path(endpoint, self.serializer.extension(), is_coll)
    }
}

impl TryFrom<&SerializerConfig> for SerializerContext {
    type Error = crate::Error;

    /// Resolves a [`SerializerConfig`] into a concrete [`SerializerContext`],
    /// looking up the serializer and layout implementations by name.
    fn try_from(config: &SerializerConfig) -> Result<Self> {
        let serializer = get_serializer(&config.serializer, config.minify)?;
        let layout = get_layout(&config.layout);
        Ok(Self {
            serializer,
            layout,
            dest: config.dest.clone(),
        })
    }
}

/// Looks up the [`Serializer`] implementation matching the given name
/// (`json`, `typescript`/`javascript`/`ts`/`js`, or `sqlite`/`sql`).
///
/// Returns [`Error::UnknownSerializer`] for any other name.
fn get_serializer(s: &str, minify: bool) -> Result<Box<dyn Serializer>> {
    match s {
        "typescript" | "javascript" | "ts" | "js" => Ok(Box::new(TypescriptSerializer { minify })),
        "sqlite" | "sql" => Ok(Box::new(SqliteSerializer)),
        "json" => Ok(Box::new(JSONSerializer { minify })),
        _ => Err(Error::UnknownSerializer(s.into())),
    }
}

/// Maps a [`Layout`] configuration value to its [`LayoutTrait`]
/// implementation.
fn get_layout(l: &Layout) -> Box<dyn LayoutTrait> {
    match l {
        Layout::File => Box::new(FileLayout),
        Layout::Extension => Box::new(ExtensionLayout),
        Layout::Index => Box::new(IndexLayout),
    }
}
