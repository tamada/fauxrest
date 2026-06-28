use std::path::PathBuf;

use serde_json::Value;

use crate::orchestrator::{ExtensionLayout, FileLayout, IndexLayout, LayoutTrait};
use crate::serializers::{JSONSerializer, SqliteSerializer, TypescriptSerializer};
use crate::{Error, Layout, Result, Serializer, SerializerConfig};

pub struct SerializerContext {
    pub serializer: Box<dyn Serializer>,
    pub layout: Box<dyn LayoutTrait>,
    pub dest: PathBuf,
}

impl SerializerContext {
    pub fn serialize(&self, data: &Value) -> Result<Vec<u8>> {
        self.serializer.serialize(data)
    }

    pub fn full_path(&self, name: &str, is_coll: bool) -> PathBuf {
        let endpoint = name.strip_suffix(".json").unwrap_or(name);
        let path = self.determine_path(endpoint, is_coll);
        self.dest.join(path)
    }

    pub fn determine_path(&self, endpoint: &str, is_coll: bool) -> PathBuf {
        self.layout
            .determine_path(endpoint, self.serializer.extension(), is_coll)
    }
}

impl TryFrom<&SerializerConfig> for SerializerContext {
    type Error = crate::Error;

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

fn get_serializer(s: &str, minify: bool) -> Result<Box<dyn Serializer>> {
    match s {
        "typescript" | "javascript" | "ts" | "js" => Ok(Box::new(TypescriptSerializer { minify })),
        "sqlite" | "sql" => Ok(Box::new(SqliteSerializer)),
        "json" => Ok(Box::new(JSONSerializer { minify })),
        _ => Err(Error::UnknownSerializer(s.into())),
    }
}

fn get_layout(l: &Layout) -> Box<dyn LayoutTrait> {
    match l {
        Layout::File => Box::new(FileLayout),
        Layout::Extension => Box::new(ExtensionLayout),
        Layout::Index => Box::new(IndexLayout),
    }
}
