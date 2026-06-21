//! Serializer implementation module

use serde_json::Value;
use rusqlite::{params, Connection};
use std::fs;
use crate::Error;

/// Trait for physical data serialization
pub trait Serializer {
    /// Serializes data into bytes
    fn serialize(&self, data: &Value) -> Result<Vec<u8>, Error>;
    /// Returns the file extension
    fn extension(&self) -> &str;
}

/// Serializes data in JSON format
pub struct JSONSerializer;
impl Serializer for JSONSerializer {
    /// Serializes data to JSON bytes
    fn serialize(&self, d: &Value) -> Result<Vec<u8>, Error> {
        serde_json::to_vec_pretty(d).map_err(|e| e.into())
    }
    /// Returns the extension 'json'
    fn extension(&self) -> &str { "json" }
}

/// Serializes data in TypeScript/JavaScript (ESM) format
pub struct TypescriptSerializer;
impl Serializer for TypescriptSerializer {
    /// Serializes data to 'export const data = ...' format
    fn serialize(&self, d: &Value) -> Result<Vec<u8>, Error> {
        let json = serde_json::to_string_pretty(d)?;
        Ok(format!("export const data = {};", json).into_bytes())
    }
    /// Returns the extension 'ts'
    fn extension(&self) -> &str { "ts" }
}

/// Serializes data as a SQLite database
pub struct SqliteSerializer;
impl Serializer for SqliteSerializer {
    /// Stores data in a temporary DB and returns its file content
    fn serialize(&self, d: &Value) -> Result<Vec<u8>, Error> {
        let tmp = tempfile::NamedTempFile::new()?;
        let conn = Connection::open(tmp.path())?;
        self.populate_db(&conn, d)?;
        fs::read(tmp.path()).map_err(|e| e.into())
    }
    /// Returns the extension 'db'
    fn extension(&self) -> &str { "db" }
}

impl SqliteSerializer {
    fn populate_db(&self, conn: &Connection, d: &Value) -> Result<(), Error> {
        conn.execute("CREATE TABLE data (id INTEGER PRIMARY KEY, value TEXT)", [])?;
        if let Some(arr) = d.as_array() {
            for (i, val) in arr.iter().enumerate() {
                conn.execute("INSERT INTO data (id, value) VALUES (?1, ?2)", params![i as i64, val.to_string()])?;
            }
        }
        Ok(())
    }
}
