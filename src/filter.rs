use std::fmt::Display;

use regex::Regex;
use serde::Deserialize;
use serde_json::Value;

use crate::{Error, Result};

/// This operation targets the `$filter` directive.
/// All operations use `op` to process the value of `field` and the given `value`.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FilterOp {
    /// `field` equals to `value`
    Eq,
    /// `field` not equals to `value`
    Neq,
    /// `field` is greater than `value`
    Gt,
    /// `field` is greater than or equals to `value`
    Gte,
    /// `field` is less than `value`
    Lt,
    /// `field` is less than or equals to `value`
    Lte,
    /// `field` contains `value`.
    Contains,
    /// `field` is exists or not (`value` should be `true` or `false`, `true` means exists)
    Exists,
    /// `field` is matched by `value`.
    RegEq,
    /// `field` is not matched by `value`.
    RegNeq,
}

impl Display for FilterOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FilterOp::Eq => write!(f, "eq"),
            FilterOp::Neq => write!(f, "neq"),
            FilterOp::Gt => write!(f, "gt"),
            FilterOp::Gte => write!(f, "gte"),
            FilterOp::Lt => write!(f, "lt"),
            FilterOp::Lte => write!(f, "lte"),
            FilterOp::Contains => write!(f, "contains"),
            FilterOp::Exists => write!(f, "exists"),
            FilterOp::RegEq => write!(f, "regeq"),
            FilterOp::RegNeq => write!(f, "regneq"),
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct FilterCondition {
    pub field: String,
    pub op: FilterOp,
    pub value: Value,
}

impl FilterCondition {
    pub fn apply(&self, item: &Value) -> Result<bool> {
        let target = item.get(&self.field);
        warn_if_type_mismatch(&self.op, &self.field, target, &self.value);
        let result = match self.op {
            FilterOp::Eq => target.is_some_and(|t| t.eq(&self.value)),
            FilterOp::Neq => target.is_some_and(|t| t.ne(&self.value)),
            FilterOp::Gt => compare_ord(target, &self.value, |lhs, rhs| lhs > rhs),
            FilterOp::Gte => compare_ord(target, &self.value, |lhs, rhs| lhs >= rhs),
            FilterOp::Lt => compare_ord(target, &self.value, |lhs, rhs| lhs < rhs),
            FilterOp::Lte => compare_ord(target, &self.value, |lhs, rhs| lhs <= rhs),
            FilterOp::Contains => contains_value(target, &self.value),
            FilterOp::Exists => {
                let expected = self.value.as_bool().unwrap_or(false);
                target.is_some() == expected
            }
            FilterOp::RegEq => regex_match(target, &self.value, true)?,
            FilterOp::RegNeq => regex_match(target, &self.value, false)?,
        };
        Ok(result)
    }
}

fn is_match_type(left: &Value, right: &Value) -> bool {
    let left_type = crate::value_kind(left);
    let right_type = crate::value_kind(right);
    left_type == right_type
}

fn compare_ord<F>(target: Option<&Value>, rhs: &Value, cmp: F) -> bool
where
    F: Fn(f64, f64) -> bool,
{
    let Some(lhs) = target else {
        return false;
    };
    if is_match_type(lhs, rhs) {
        cmp(lhs.as_f64().unwrap_or(0.0), rhs.as_f64().unwrap_or(0.0))
    } else {
        false
    }
}

fn contains_value(target: Option<&Value>, rhs: &Value) -> bool {
    let Some(target) = target else {
        return false;
    };
    match target {
        Value::String(s) => rhs
            .as_str()
            .map(|needle| s.contains(needle))
            .unwrap_or(false),
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

fn warn_if_type_mismatch(op: &FilterOp, field: &str, target: Option<&Value>, right: &Value) {
    let Some(left) = target else {
        return;
    };
    if !is_type_compatible(op, left, right) {
        crate::emit_type_mismatch_warning(op, field, left, right);
    }
}

fn is_type_compatible(op: &FilterOp, lhs: &Value, rhs: &Value) -> bool {
    use FilterOp::*;
    match op {
        Eq | Neq => crate::value_kind(lhs) == crate::value_kind(rhs),
        Gt | Gte | Lt | Lte => lhs.is_number() && rhs.is_number(),
        Contains => match lhs {
            Value::String(_) => rhs.is_string(),
            Value::Array(_) => true,
            _ => false,
        },
        RegEq | RegNeq => lhs.is_string() && rhs.is_string(),
        _ => true,
    }
}
