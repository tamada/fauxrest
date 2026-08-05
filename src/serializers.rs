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

    /// Serializes every endpoint of a bundled build into one file, given a map
    /// of endpoint path to payload.
    ///
    /// The default treats the map as an ordinary object, which is what a
    /// caller of the JSON or TypeScript output wants: one value keyed by path.
    /// [`SqliteSerializer`] overrides it, because a database of one row per
    /// endpoint is more use than a database holding a single nested object.
    ///
    /// # Examples
    ///
    /// ```
    /// use fauxrest::{JSONSerializer, Serializer};
    /// use serde_json::{Map, json};
    ///
    /// let mut endpoints = Map::new();
    /// endpoints.insert("/users".to_string(), json!([{ "id": 1 }]));
    ///
    /// let serializer = JSONSerializer { minify: true };
    /// let bytes = serializer.serialize_bundle(&endpoints).unwrap();
    /// assert_eq!(bytes, br#"{"/users":[{"id":1}]}"#);
    /// ```
    fn serialize_bundle(
        &self,
        endpoints: &serde_json::Map<String, Value>,
    ) -> Result<Vec<u8>, Error> {
        self.serialize(&Value::Object(endpoints.clone()))
    }
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
        self.build_db(|conn| self.populate_db(conn, d))
    }
    /// Returns the extension 'db'
    fn extension(&self) -> &str {
        "db"
    }

    /// Writes one row per endpoint into `endpoints(path TEXT PRIMARY KEY,
    /// value TEXT)`, so a bundled build is a database that can be queried by
    /// path rather than one holding a single nested object.
    fn serialize_bundle(
        &self,
        endpoints: &serde_json::Map<String, Value>,
    ) -> Result<Vec<u8>, Error> {
        self.build_db(|conn| {
            conn.execute(
                "CREATE TABLE endpoints (path TEXT PRIMARY KEY, value TEXT)",
                [],
            )?;
            for (path, value) in endpoints {
                conn.execute(
                    "INSERT INTO endpoints (path, value) VALUES (?1, ?2)",
                    params![path, value.to_string()],
                )?;
            }
            Ok(())
        })
    }
}

impl SqliteSerializer {
    /// Builds a database with `populate`, returning its file content.
    ///
    /// `rusqlite` writes to a path rather than to memory, so the database is
    /// assembled in a temporary file and read back.
    fn build_db<F>(&self, populate: F) -> Result<Vec<u8>, Error>
    where
        F: FnOnce(&Connection) -> Result<(), Error>,
    {
        let tmp = tempfile::NamedTempFile::new()?;
        let conn = Connection::open(tmp.path())?;
        populate(&conn)?;
        fs::read(tmp.path()).map_err(|e| e.into())
    }

    /// Creates a `data(id INTEGER PRIMARY KEY, value TEXT)` table in `conn`
    /// and fills it with the endpoint's payload as JSON text.
    ///
    /// An array becomes one row per element, keyed by index. Anything else —
    /// an object endpoint such as `/profile`, or a bare scalar — becomes a
    /// single row. It used to produce an empty table instead, so a `sqlite`
    /// build silently dropped every endpoint that was not a collection.
    fn populate_db(&self, conn: &Connection, d: &Value) -> Result<(), Error> {
        conn.execute("CREATE TABLE data (id INTEGER PRIMARY KEY, value TEXT)", [])?;
        match d.as_array() {
            Some(arr) => {
                for (i, val) in arr.iter().enumerate() {
                    conn.execute(
                        "INSERT INTO data (id, value) VALUES (?1, ?2)",
                        params![i as i64, val.to_string()],
                    )?;
                }
            }
            None => {
                conn.execute(
                    "INSERT INTO data (id, value) VALUES (?1, ?2)",
                    params![0i64, d.to_string()],
                )?;
            }
        }
        Ok(())
    }
}
