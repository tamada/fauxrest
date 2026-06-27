//! Compilation orchestrator module
//!
//! Orchestrates the multi-serializer execution loop based on configuration,
//! reading raw JSON and generating static files according to specified layouts.

use regex::Regex;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use serde_json::{json, Map, Value};

use crate::config::{ApiNode, DeriveConfig, FilterCondition, FilterOp};
use crate::{Config, Error, JSONSerializer, Layout, Result, Serializer, SerializerConfig, SqliteSerializer, TypescriptSerializer};

const DERIVE_CARDINALITY_WARN_THRESHOLD: usize = 1000;

/// Executes the API build process
pub fn run<P: AsRef<Path>>(config: Config, data_dir: P) -> Result<()> {
    let mut endpoints = Vec::new();
    for s_conf in &config.serializers {
        endpoints.extend(run_serializer(s_conf, data_dir.as_ref(), &config.api)?);
    }
    generate_discovery(&config, &endpoints)?;
    Ok(())
}

fn run_serializer(
    conf: &SerializerConfig,
    data_dir: &Path,
    api_overlay: &HashMap<String, ApiNode>,
) -> Result<Vec<String>> {
    let serializer = get_serializer(&conf.serializer, conf.minify)?;
    let layout = get_layout(&conf.layout);
    let datasets = collect_datasets(data_dir)?;
    let mut names = datasets.keys().cloned().collect::<Vec<_>>();
    names.sort();
    let mut endpoints = Vec::new();

    for name in names {
        let data = datasets
            .get(&name)
            .ok_or_else(|| Error::Config(format!("missing dataset for endpoint '{}'", name)))?;
        let node = api_overlay.get(&name);
        endpoints.extend(materialize_node(
            &name,
            data,
            node,
            None,
            &datasets,
            serializer.as_ref(),
            layout.as_ref(),
            &conf.dest,
        )?);
    }
    endpoints.sort();
    endpoints.dedup();
    Ok(endpoints)
}

fn collect_datasets(data_dir: &Path) -> Result<HashMap<String, Value>> {
    let mut datasets = HashMap::new();
    let entries = fs::read_dir(data_dir).map_err(Error::Io)?;
    for entry in entries {
        let entry = entry.map_err(Error::Io)?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();
        if file_name_str.starts_with('_') || file_name_str.starts_with('.') {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let content = fs::read_to_string(&path).map_err(Error::Io)?;
        let json: Value = serde_json::from_str(&content).map_err(Error::SerdeJson)?;
        let name = file_name_str
            .strip_suffix(".json")
            .unwrap_or(&file_name_str)
            .to_string();
        datasets.insert(name, json);
    }
    Ok(datasets)
}

#[allow(clippy::too_many_arguments)]
fn materialize_node(
    endpoint: &str,
    source_data: &Value,
    node: Option<&ApiNode>,
    inherited_filter: Option<&Vec<FilterCondition>>,
    datasets: &HashMap<String, Value>,
    s: &dyn Serializer,
    l: &dyn LayoutTrait,
    dest: &Path,
) -> Result<Vec<String>> {
    let mut endpoint_base = source_data.clone();
    if let Some(agg) = node.and_then(|n| n.aggregate.as_ref()) {
        endpoint_base = aggregate_values(agg, datasets)?;
    }

    let effective_filter = node
        .and_then(|n| n.filter.as_ref())
        .or(inherited_filter);
    let mut endpoint_data = if let Some(filters) = effective_filter {
        apply_filters(&endpoint_base, filters)?
    } else {
        endpoint_base.clone()
    };

    if let Some(n) = node {
        endpoint_data = apply_pick_omit(endpoint_data, n);
    }

    let is_private = node.and_then(|n| n.private).unwrap_or(false);
    if is_private {
        return Ok(vec![]);
    }
    write_data(endpoint, &endpoint_data, s, l, dest)?;

    let mut endpoints = vec![format!("/{}", endpoint)];

    if let Value::Array(arr) = &endpoint_data {
        for item in arr {
            if let Some(id) = item.get("id") {
                let id_str = id.as_str().map(|s| s.to_string()).unwrap_or_else(|| id.to_string());
                let item_path = format!("{}/{}", endpoint, id_str);
                write_data(&item_path, item, s, l, dest)?;
                endpoints.push(format!("/{}", item_path));
            }
        }
    }

    if let Some(current) = node {
        let mut keys = current.sub_paths.keys().cloned().collect::<Vec<_>>();
        keys.sort();
        for key in keys {
            let child = current
                .sub_paths
                .get(&key)
                .ok_or_else(|| Error::Config(format!("missing sub-path node '{}'", key)))?;
            if let Some(var) = template_var_from_key(&key) {
                let values = resolve_template_values(endpoint, &key, child, &endpoint_base)?;
                for value in &values {
                    let segment = scalar_to_path_segment(value)?;
                    let child_endpoint = format!("{}/{}", endpoint, segment);
                    let expanded_child = expand_template_node(child, var, value);
                    endpoints.extend(materialize_node(
                        &child_endpoint,
                        &endpoint_base,
                        Some(&expanded_child),
                        effective_filter,
                        datasets,
                        s,
                        l,
                        dest,
                    )?);
                }
            } else {
                let child_endpoint = format!("{}/{}", endpoint, key);
                endpoints.extend(materialize_node(
                    &child_endpoint,
                    &endpoint_base,
                    Some(child),
                    effective_filter,
                    datasets,
                    s,
                    l,
                    dest,
                )?);
            }
        }
    }

    Ok(endpoints)
}

fn aggregate_values(paths: &[String], datasets: &HashMap<String, Value>) -> Result<Value> {
    let mut merged = Vec::new();
    for path in paths {
        let data = datasets
            .get(path)
            .ok_or_else(|| Error::Config(format!("$aggregate references unknown dataset '{}'", path)))?;
        if let Value::Array(arr) = data {
            merged.extend(arr.iter().cloned());
        } else {
            merged.push(data.clone());
        }
    }
    Ok(Value::Array(merged))
}

fn apply_filters(data: &Value, filters: &[FilterCondition]) -> Result<Value> {
    match data {
        Value::Array(arr) => {
            let mut out = Vec::new();
            for item in arr {
                if matches_all_conditions(item, filters)? {
                    out.push(item.clone());
                }
            }
            Ok(Value::Array(out))
        }
        _ => {
            if matches_all_conditions(data, filters)? {
                Ok(data.clone())
            } else {
                Ok(Value::Null)
            }
        }
    }
}

fn matches_all_conditions(item: &Value, filters: &[FilterCondition]) -> Result<bool> {
    for cond in filters {
        if !matches_condition(item, cond)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn matches_condition(item: &Value, cond: &FilterCondition) -> Result<bool> {
    let target = item.get(&cond.field);
    let result = match cond.op {
        FilterOp::Eq => target == Some(&cond.value),
        FilterOp::Neq => target != Some(&cond.value),
        FilterOp::Gt => compare_ord(target, &cond.value, |lhs, rhs| lhs > rhs),
        FilterOp::Gte => compare_ord(target, &cond.value, |lhs, rhs| lhs >= rhs),
        FilterOp::Lt => compare_ord(target, &cond.value, |lhs, rhs| lhs < rhs),
        FilterOp::Lte => compare_ord(target, &cond.value, |lhs, rhs| lhs <= rhs),
        FilterOp::Contains => contains_value(target, &cond.value),
        FilterOp::Exists => {
            let expected = cond.value.as_bool().unwrap_or(false);
            target.is_some() == expected
        }
        FilterOp::RegEq => regex_match(target, &cond.value, true)?,
        FilterOp::RegNeq => regex_match(target, &cond.value, false)?,
    };
    Ok(result)
}

fn compare_ord<F>(target: Option<&Value>, rhs: &Value, cmp: F) -> bool
where
    F: Fn(f64, f64) -> bool,
{
    let Some(lhs) = target.and_then(Value::as_f64) else {
        return false;
    };
    let Some(rhs_num) = rhs.as_f64() else {
        return false;
    };
    cmp(lhs, rhs_num)
}

fn contains_value(target: Option<&Value>, rhs: &Value) -> bool {
    let Some(target) = target else {
        return false;
    };
    match target {
        Value::String(s) => rhs.as_str().map(|needle| s.contains(needle)).unwrap_or(false),
        Value::Array(arr) => arr.iter().any(|v| v == rhs),
        _ => false,
    }
}

fn regex_match(target: Option<&Value>, rhs: &Value, positive: bool) -> Result<bool> {
    let Some(value) = target.and_then(Value::as_str) else {
        return Ok(!positive);
    };
    let pattern = rhs
        .as_str()
        .ok_or_else(|| Error::Config("regex filter value must be a string".to_string()))?;
    let re = Regex::new(pattern)
        .map_err(|e| Error::Config(format!("invalid regex '{}': {}", pattern, e)))?;
    let matched = re.is_match(value);
    Ok(if positive { matched } else { !matched })
}

fn apply_pick_omit(mut value: Value, node: &ApiNode) -> Value {
    if let Some(pick) = &node.pick {
        value = match value {
            Value::Object(obj) => Value::Object(apply_pick_map(obj, pick)),
            Value::Array(arr) => Value::Array(
                arr.into_iter()
                    .map(|item| match item {
                        Value::Object(obj) => Value::Object(apply_pick_map(obj, pick)),
                        _ => item,
                    })
                    .collect(),
            ),
            other => other,
        };
    }
    if let Some(omit) = &node.omit {
        value = match value {
            Value::Object(obj) => Value::Object(apply_omit_map(obj, omit)),
            Value::Array(arr) => Value::Array(
                arr.into_iter()
                    .map(|item| match item {
                        Value::Object(obj) => Value::Object(apply_omit_map(obj, omit)),
                        _ => item,
                    })
                    .collect(),
            ),
            other => other,
        };
    }
    value
}

fn apply_pick_map(mut obj: Map<String, Value>, pick: &[String]) -> Map<String, Value> {
    obj.retain(|k, _| pick.contains(k));
    obj
}

fn apply_omit_map(mut obj: Map<String, Value>, omit: &[String]) -> Map<String, Value> {
    for key in omit {
        obj.remove(key);
    }
    obj
}

fn resolve_template_values(
    endpoint: &str,
    key: &str,
    child: &ApiNode,
    source_data: &Value,
) -> Result<Vec<Value>> {
    if let Some(values) = child.values.as_ref() {
        return Ok(values.clone());
    }

    if let Some(derive) = child.derive.as_ref() {
        let cfg = derive.to_config();
        return derive_values_from_data(source_data, &cfg, &format!("{}/{}", endpoint, key));
    }

    Err(Error::Config(format!(
        "{}/{}: template key requires $values or $derive",
        endpoint, key
    )))
}

fn derive_values_from_data(source_data: &Value, cfg: &DeriveConfig, context: &str) -> Result<Vec<Value>> {
    let mut unique: BTreeMap<String, Value> = BTreeMap::new();
    let mut skipped = 0usize;

    match source_data {
        Value::Array(arr) => {
            for item in arr {
                if let Some(v) = item.get(&cfg.field) {
                    if let Some(extracted) = derive_scalar_value(v, cfg)? {
                        let key = scalar_deterministic_key(&extracted);
                        unique.entry(key).or_insert(extracted);
                    } else {
                        skipped += 1;
                    }
                }
            }
        }
        Value::Object(obj) => {
            if let Some(v) = obj.get(&cfg.field) {
                if let Some(extracted) = derive_scalar_value(v, cfg)? {
                    let key = scalar_deterministic_key(&extracted);
                    unique.entry(key).or_insert(extracted);
                } else {
                    skipped += 1;
                }
            }
        }
        _ => {
            return Ok(vec![]);
        }
    }

    if unique.len() > DERIVE_CARDINALITY_WARN_THRESHOLD {
        eprintln!(
            "warning: {} derived {} values (threshold {})",
            context,
            unique.len(),
            DERIVE_CARDINALITY_WARN_THRESHOLD
        );
    }
    if skipped > 0 {
        eprintln!(
            "warning: {} skipped {} non-derivable values while processing $derive",
            context,
            skipped
        );
    }

    Ok(unique.into_values().collect())
}

fn derive_scalar_value(value: &Value, cfg: &DeriveConfig) -> Result<Option<Value>> {
    let extracted = if let Some(pattern) = cfg.pattern.as_ref() {
        let Some(s) = value.as_str() else {
            return Ok(None);
        };
        let re = Regex::new(pattern)
            .map_err(|e| Error::Config(format!("invalid $derive.pattern '{}': {}", pattern, e)))?;
        if let Some(caps) = re.captures(s) {
            if let Some(group1) = caps.get(1) {
                Value::String(group1.as_str().to_string())
            } else if let Some(full) = caps.get(0) {
                Value::String(full.as_str().to_string())
            } else {
                return Ok(None);
            }
        } else {
            return Ok(None);
        }
    } else {
        value.clone()
    };

    if !matches!(extracted, Value::String(_) | Value::Number(_) | Value::Bool(_)) {
        return Ok(None);
    }
    if let Value::String(ref s) = extracted {
        if s.is_empty() || s.contains('/') {
            return Ok(None);
        }
    }
    Ok(Some(extracted))
}

fn scalar_deterministic_key(value: &Value) -> String {
    match value {
        Value::String(s) => format!("s:{}", s),
        Value::Number(n) => format!("n:{}", n),
        Value::Bool(b) => format!("b:{}", b),
        _ => format!("x:{}", value),
    }
}

fn template_var_from_key(key: &str) -> Option<&str> {
    if key.starts_with("${") && key.ends_with('}') && key.len() > 3 {
        Some(&key[2..key.len() - 1])
    } else {
        None
    }
}

fn expand_template_node(node: &ApiNode, var: &str, value: &Value) -> ApiNode {
    let mut out = node.clone();
    out.values = None;
    out.derive = None;
    if let Some(filters) = &node.filter {
        let token = format!("{{{}}}", var);
        out.filter = Some(
            filters
                .iter()
                .map(|cond| FilterCondition {
                    field: cond.field.clone(),
                    op: cond.op.clone(),
                    value: replace_template_token(&cond.value, &token, value),
                })
                .collect(),
        );
    }
    out
}

fn replace_template_token(input: &Value, token: &str, replacement: &Value) -> Value {
    match input {
        Value::String(s) => {
            if s == token {
                replacement.clone()
            } else if s.contains(token) {
                Value::String(s.replace(token, &scalar_to_string(replacement)))
            } else {
                Value::String(s.clone())
            }
        }
        Value::Array(arr) => Value::Array(
            arr.iter()
                .map(|v| replace_template_token(v, token, replacement))
                .collect(),
        ),
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                out.insert(k.clone(), replace_template_token(v, token, replacement));
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

fn scalar_to_path_segment(value: &Value) -> Result<String> {
    let segment = scalar_to_string(value);
    if segment.is_empty() || segment.contains('/') {
        return Err(Error::Config(format!(
            "template value '{}' cannot be used as a path segment",
            segment
        )));
    }
    Ok(segment)
}

fn scalar_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => value.to_string(),
    }
}

fn write_data(
    name: &str,
    data: &Value,
    s: &dyn Serializer,
    l: &dyn LayoutTrait,
    dest: &Path
) -> Result<()> {
    let is_coll = data.is_array();
    let endpoint = name.strip_suffix(".json").unwrap_or(name);
    let path = l.determine_path(endpoint, s.extension(), is_coll);
    let full_path = dest.join(path);
    
    fs::create_dir_all(full_path.parent().unwrap()).map_err(Error::Io)?;
    fs::write(full_path, s.serialize(data)?).map_err(Error::Io)?;
    Ok(())
}

fn generate_discovery(config: &Config, endpoints: &[String]) -> Result<()> {
    let discovery = json!({ "endpoints": endpoints });
    for s_conf in &config.serializers {
        let s = get_serializer(&s_conf.serializer, s_conf.minify)?;
        let path = s_conf.dest.join(format!("index.{}", s.extension()));
        fs::create_dir_all(path.parent().unwrap()).map_err(Error::Io)?;
        fs::write(path, s.serialize(&discovery)?).map_err(Error::Io)?;
    }
    Ok(())
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

/// Trait for physical data serialization
trait LayoutTrait {
    fn determine_path(&self, endpoint: &str, file_ext: &str, is_coll: bool) -> PathBuf;
}

/// Layout that places files in `index.[ext]`
struct IndexLayout;
impl LayoutTrait for IndexLayout {
    fn determine_path(&self, endpoint: &str, ext: &str, _: bool) -> PathBuf {
        Path::new(endpoint).join(format!("index.{}", ext))
    }
}

/// Layout that appends the extension directly
struct ExtensionLayout;
impl LayoutTrait for ExtensionLayout {
    fn determine_path(&self, endpoint: &str, ext: &str, _: bool) -> PathBuf {
        PathBuf::from(format!("{}.{}", endpoint, ext))
    }
}

/// Layout that avoids extensions where possible (supports smart fallback)
struct FileLayout;
impl FileLayout {
    fn is_coll_path(&self, endpoint: &str, is_coll: bool) -> bool {
        is_coll && !endpoint.is_empty()
    }
}
impl LayoutTrait for FileLayout {
    fn determine_path(&self, endpoint: &str, ext: &str, is_coll: bool) -> PathBuf {
        if self.is_coll_path(endpoint, is_coll) {
            Path::new(endpoint).join(format!("index.{}", ext))
        } else {
            PathBuf::from(endpoint)
        }
    }
}
