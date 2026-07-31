//! Serializer implementation module
//!
//! Provides the [`Serializer`] trait and its concrete implementations
//! ([`JSONSerializer`], [`TypescriptSerializer`], [`SqliteSerializer`]) that
//! turn a `serde_json::Value` into the bytes written for each endpoint.

use crate::Error;
use rusqlite::{Connection, params};
use serde_json::Value;
use std::fs;

/// Trait for physical data serialization
pub trait Serializer {
    /// Serializes data into bytes
    ///
    /// # Examples
    ///
    /// ```
    /// use fauxrest::{JSONSerializer, Serializer};
    /// use serde_json::json;
    ///
    /// let serializer = JSONSerializer { minify: true };
    /// let bytes = serializer.serialize(&json!({ "id": 1 })).unwrap();
    /// assert_eq!(bytes, br#"{"id":1}"#);
    /// ```
    fn serialize(&self, data: &Value) -> Result<Vec<u8>, Error>;
    /// Returns the file extension
    ///
    /// # Examples
    ///
    /// ```
    /// use fauxrest::{JSONSerializer, Serializer};
    ///
    /// let serializer = JSONSerializer { minify: false };
    /// assert_eq!(serializer.extension(), "json");
    /// ```
    fn extension(&self) -> &str;
}

/// Serializes data in JSON format
pub struct JSONSerializer {
    /// If `true`, output compact (minified) JSON; otherwise pretty-print.
    pub minify: bool,
}
impl Serializer for JSONSerializer {
    /// Serializes data to JSON bytes
    fn serialize(&self, d: &Value) -> Result<Vec<u8>, Error> {
        if self.minify {
            serde_json::to_vec(d).map_err(|e| e.into())
        } else {
            serde_json::to_vec_pretty(d).map_err(|e| e.into())
        }
    }
    /// Returns the extension 'json'
    fn extension(&self) -> &str {
        "json"
    }
}

/// Serializes data in TypeScript/JavaScript (ESM) format
pub struct TypescriptSerializer {
    /// If `true`, output compact (minified) JSON inside the module;
    /// otherwise pretty-print.
    pub minify: bool,
}
impl Serializer for TypescriptSerializer {
    /// Serializes data to 'export const data = ...' format
    fn serialize(&self, d: &Value) -> Result<Vec<u8>, Error> {
        let json = if self.minify {
            serde_json::to_string(d)?
        } else {
            serde_json::to_string_pretty(d)?
        };
        Ok(format!("export const data = {};", json).into_bytes())
    }
    /// Returns the extension 'ts'
    fn extension(&self) -> &str {
        "ts"
    }
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
    fn extension(&self) -> &str {
        "db"
    }
}

impl SqliteSerializer {
    /// Creates a `data(id INTEGER PRIMARY KEY, value TEXT)` table in `conn`
    /// and inserts one row per array element of `d` (its JSON string form
    /// as `value`, its index as `id`). If `d` is not an array, no rows are
    /// inserted and only the empty table is created.
    fn populate_db(&self, conn: &Connection, d: &Value) -> Result<(), Error> {
        conn.execute("CREATE TABLE data (id INTEGER PRIMARY KEY, value TEXT)", [])?;
        if let Some(arr) = d.as_array() {
            for (i, val) in arr.iter().enumerate() {
                conn.execute(
                    "INSERT INTO data (id, value) VALUES (?1, ?2)",
                    params![i as i64, val.to_string()],
                )?;
            }
        }
        Ok(())
    }
}
