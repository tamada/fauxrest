use std::fs;
use std::path::{Path, PathBuf};
use serde_json::Value;

/// A Static API Generator that compiles raw JSON datasets into structured directories of static files.
///
/// # Examples
///
/// ```
/// use prest::Generator;
/// use std::path::Path;
///
/// let generator = Generator::new("testdata", "dist_temp");
/// assert!(true);
/// ```
pub struct Generator {
    inputs: PathBuf,
    dest: PathBuf,
}

impl Generator {
    /// Creates a new `Generator` with source inputs and destination path.
    ///
    /// # Examples
    ///
    /// ```
    /// use prest::Generator;
    /// let generator = Generator::new("testdata", "dist_temp");
    /// ```
    pub fn new<P1: AsRef<Path>, P2: AsRef<Path>>(inputs: P1, dest: P2) -> Self {
        Self {
            inputs: inputs.as_ref().to_path_buf(),
            dest: dest.as_ref().to_path_buf(),
        }
    }

    /// Compiles inputs into the static destination directory.
    ///
    /// # Examples
    ///
    /// ```
    /// use prest::Generator;
    /// let generator = Generator::new("testdata", "dist_temp");
    /// // generator.generate().unwrap();
    /// ```
    pub fn generate(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.validate_inputs()?;
        self.ensure_dest_exists()?;
        self.process_all_sources()?;
        Ok(())
    }

    /// Validates that the inputs path actually exists.
    fn validate_inputs(&self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.inputs.exists() {
            return Err(format!("Inputs path does not exist: {:?}", self.inputs).into());
        }
        Ok(())
    }

    /// Ensures that the destination directory exists.
    fn ensure_dest_exists(&self) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(&self.dest)?;
        Ok(())
    }

    /// Dispatches processing depending on whether inputs is a file or directory.
    fn process_all_sources(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.inputs.is_file() {
            self.process_file(&self.inputs)
        } else {
            self.process_directory()
        }
    }

    /// Processes all JSON files in the inputs directory.
    fn process_directory(&self) -> Result<(), Box<dyn std::error::Error>> {
        for entry in fs::read_dir(&self.inputs)? {
            let path = entry?.path();
            if self.is_json_file(&path) {
                self.process_file(&path)?;
            }
        }
        Ok(())
    }

    /// Checks if a given path is a valid JSON file.
    fn is_json_file(&self, path: &Path) -> bool {
        path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json")
    }

    /// Processes a single JSON file and determines the routing transformation.
    fn process_file(&self, file_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let name = self.extract_file_stem(file_path)?;
        let content = fs::read_to_string(file_path)?;
        let json: Value = serde_json::from_str(&content)?;
        self.route_json(&name, &json)
    }

    /// Extracts the file stem as a string.
    fn extract_file_stem(&self, path: &Path) -> Result<String, Box<dyn std::error::Error>> {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(String::from)
            .ok_or_else(|| "Invalid file name".into())
    }

    /// Routes JSON data based on its shape (Object or Array).
    fn route_json(&self, name: &str, json: &Value) -> Result<(), Box<dyn std::error::Error>> {
        match json {
            Value::Object(map) => self.write_endpoint(name, &Value::Object(map.clone())),
            Value::Array(arr) => self.route_array(name, arr),
            _ => self.write_endpoint(name, json),
        }
    }

    /// Routes a JSON array as a list and processes individual items for detail view.
    fn route_array(&self, name: &str, arr: &[Value]) -> Result<(), Box<dyn std::error::Error>> {
        self.write_endpoint(name, &Value::Array(arr.to_vec()))?;
        for item in arr {
            self.route_array_item(name, item)?;
        }
        Ok(())
    }

    /// Processes a single item in an array, extracting ID if present to write detail.
    fn route_array_item(&self, parent_name: &str, item: &Value) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(id_str) = self.extract_id(item) {
            let item_path = format!("{}/{}", parent_name, id_str);
            self.write_endpoint(&item_path, item)?;
        }
        Ok(())
    }

    /// Extracts an "id" field as string from a JSON Value if it exists.
    fn extract_id(&self, value: &Value) -> Option<String> {
        let map = value.as_object()?;
        let id_val = map.get("id")?;
        match id_val {
            Value::Number(n) => Some(n.to_string()),
            Value::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    /// Writes a JSON value to a designated index.json endpoint.
    fn write_endpoint(&self, relative_path: &str, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
        let endpoint_dir = self.dest.join(relative_path);
        fs::create_dir_all(&endpoint_dir)?;
        let index_file = endpoint_dir.join("index.json");
        let serialized = serde_json::to_string_pretty(value)?;
        fs::write(index_file, serialized)?;
        Ok(())
    }
}
