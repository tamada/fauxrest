//! `$filter` directive support: the [`FilterOp`] comparison operators and
//! [`FilterCondition`](crate::filter::FilterCondition) evaluation used to
//! keep or drop items when materializing an endpoint.

use std::cmp::Ordering;
use std::fmt::Display;

use serde::Deserialize;
use serde_json::Value;

use crate::{Error, Result};

/// This operation targets the `$filter` directive.
/// All operations use `op` to process the value of `field` and the given `value`.
///
/// The ordering operators accept two numbers or two strings; strings compare
/// lexicographically, which is what sorts ISO-8601 dates correctly. Mixing
/// kinds does not match and reports a type-mismatch warning.
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
    /// Formats this operator using its lowercase JSON name (e.g. `"eq"`,
    /// `"regeq"`), matching the strings used in `_config.json`.
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

/// A single `$filter` condition: `field <op> value`, evaluated against each
/// candidate item.
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct FilterCondition {
    /// Name of the object field to read from each item.
    pub field: String,
    /// Comparison operator to apply.
    pub op: FilterOp,
    /// Right-hand side value to compare the field against.
    pub value: Value,
}

impl FilterCondition {
    /// Evaluates this condition against `item`, returning whether it
    /// matches.
    ///
    /// If `item` does not have `field`, the condition is considered
    /// unmatched for every operator except [`FilterOp::Exists`] (which
    /// checks for the field's presence) and [`FilterOp::RegNeq`] (which
    /// treats a missing field as not matching the pattern, i.e. `true`).
    /// Returns an error if [`FilterOp::RegEq`]/[`FilterOp::RegNeq`] is used
    /// with a non-string `value` or an invalid regex pattern.
    ///
    /// # Examples
    ///
    /// ```
    /// use fauxrest::filter::{FilterCondition, FilterOp};
    /// use serde_json::json;
    ///
    /// let cond = FilterCondition {
    ///     field: "status".to_string(),
    ///     op: FilterOp::Eq,
    ///     value: json!("active"),
    /// };
    /// let item = json!({ "status": "active" });
    /// assert!(cond.apply(&item).unwrap());
    /// ```
    pub fn apply(&self, item: &Value) -> Result<bool> {
        let target = item.get(&self.field);
        warn_if_type_mismatch(&self.op, &self.field, target, &self.value);
        let result = match self.op {
            FilterOp::Eq => target.is_some_and(|t| t.eq(&self.value)),
            FilterOp::Neq => target.is_some_and(|t| t.ne(&self.value)),
            FilterOp::Gt => compare_ord(target, &self.value, Ordering::is_gt),
            FilterOp::Gte => compare_ord(target, &self.value, Ordering::is_ge),
            FilterOp::Lt => compare_ord(target, &self.value, Ordering::is_lt),
            FilterOp::Lte => compare_ord(target, &self.value, Ordering::is_le),
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

/// Implements the ordering operators (`gt`, `gte`, `lt`, `lte`): orders
/// `target` against `rhs` with [`scalar_ordering`] and asks `accept` whether
/// that ordering satisfies the operator. Returns `false` if `target` is absent
/// or the two cannot be ordered.
fn compare_ord<F>(target: Option<&Value>, rhs: &Value, accept: F) -> bool
where
    F: Fn(Ordering) -> bool,
{
    let Some(lhs) = target else {
        return false;
    };
    scalar_ordering(lhs, rhs).is_some_and(accept)
}

/// Orders two values of the same JSON kind: numbers numerically, strings
/// lexicographically by Unicode scalar value. Returns `None` when the kinds
/// differ, or for kinds that have no useful order (bool, null, array,
/// object).
///
/// Lexicographic order is what makes zero-padded, fixed-width formats such as
/// ISO-8601 dates (`"2026-10-16"`) sort correctly as strings. It is *not*
/// meaningful for free-form text like `"April 2023"`, so ordering such a field
/// gives an answer that is well-defined but not the one a reader would expect.
fn scalar_ordering(lhs: &Value, rhs: &Value) -> Option<Ordering> {
    match (lhs, rhs) {
        (Value::Number(_), Value::Number(_)) => lhs.as_f64()?.partial_cmp(&rhs.as_f64()?),
        (Value::String(lhs), Value::String(rhs)) => Some(lhs.as_str().cmp(rhs.as_str())),
        _ => None,
    }
}

/// Implements the `contains` operator: for a string `target`, checks for a
/// substring match against `rhs`; for an array `target`, checks whether any
/// element equals `rhs`. Returns `false` for any other target kind or if
/// `target` is absent.
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

/// Implements the `regeq`/`regneq` operators: compiles `rhs` as a regex and
/// matches it against `target` (which must be a string). `positive`
/// selects between `regeq` semantics (`true` = matched) and `regneq`
/// semantics (`true` = not matched). A missing or non-string `target` is
/// treated as "did not match".
fn regex_match(target: Option<&Value>, rhs: &Value, positive: bool) -> Result<bool> {
    let Some(value) = target.and_then(Value::as_str) else {
        return Ok(!positive);
    };
    let pattern = rhs
        .as_str()
        .ok_or_else(|| Error::Config("regex filter value must be a string".to_string()))?;
    let re = crate::compile_regex(pattern)
        .map_err(|e| Error::Config(format!("invalid regex '{}': {}", pattern, e)))?;
    let matched = re.is_match(value);
    Ok(if positive { matched } else { !matched })
}

/// Emits a one-time diagnostic warning (via
/// [`crate::emit_type_mismatch_warning`]) when `target` is present but its
/// JSON kind is incompatible with `right` for the given operator. No-op if
/// `target` is absent.
fn warn_if_type_mismatch(op: &FilterOp, field: &str, target: Option<&Value>, right: &Value) {
    let Some(left) = target else {
        return;
    };
    if !is_type_compatible(op, left, right) {
        crate::emit_type_mismatch_warning(op, field, left, right);
    }
}

/// Returns whether `lhs` and `rhs` have JSON kinds that make sense to
/// compare under `op` (e.g. both numeric or both strings for ordering
/// operators, both strings for regex operators). Operators without a specific
/// kind requirement (like `exists`) are always considered compatible.
///
/// This drives the stderr type-mismatch warning only, so it has to agree with
/// what the operators actually support — reporting a comparison the evaluator
/// handles correctly would be noise.
fn is_type_compatible(op: &FilterOp, lhs: &Value, rhs: &Value) -> bool {
    use FilterOp::*;
    match op {
        Eq | Neq => crate::value_kind(lhs) == crate::value_kind(rhs),
        Gt | Gte | Lt | Lte => {
            (lhs.is_number() && rhs.is_number()) || (lhs.is_string() && rhs.is_string())
        }
        Contains => match lhs {
            Value::String(_) => rhs.is_string(),
            Value::Array(_) => true,
            _ => false,
        },
        RegEq | RegNeq => lhs.is_string() && rhs.is_string(),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cond(field: &str, op: FilterOp, value: Value) -> FilterCondition {
        FilterCondition {
            field: field.to_string(),
            op,
            value,
        }
    }

    /// Strings order lexicographically, which is what makes ISO-8601 dates
    /// comparable. Before this worked, `gt`/`lt` matched nothing and
    /// `gte`/`lte` matched everything, because both operands collapsed to
    /// `0.0`.
    #[test]
    fn test_iso_dates_compare_as_strings() {
        let early = json!({ "from": "2018-04-01" });
        let late = json!({ "from": "2026-10-16" });
        let pivot = json!("2020-01-01");

        assert!(
            cond("from", FilterOp::Gt, pivot.clone())
                .apply(&late)
                .unwrap()
        );
        assert!(
            !cond("from", FilterOp::Gt, pivot.clone())
                .apply(&early)
                .unwrap()
        );
        assert!(
            cond("from", FilterOp::Lt, pivot.clone())
                .apply(&early)
                .unwrap()
        );
        assert!(
            !cond("from", FilterOp::Lt, pivot.clone())
                .apply(&late)
                .unwrap()
        );
    }

    /// The inclusive operators must distinguish equal from unequal rather than
    /// accepting everything.
    #[test]
    fn test_inclusive_operators_are_not_always_true() {
        let item = json!({ "from": "2020-01-01" });
        let same = json!("2020-01-01");
        let later = json!("2021-01-01");

        assert!(
            cond("from", FilterOp::Gte, same.clone())
                .apply(&item)
                .unwrap()
        );
        assert!(cond("from", FilterOp::Lte, same).apply(&item).unwrap());
        assert!(
            !cond("from", FilterOp::Gte, later.clone())
                .apply(&item)
                .unwrap()
        );
        assert!(cond("from", FilterOp::Lte, later).apply(&item).unwrap());
    }

    /// Numeric ordering is unchanged.
    #[test]
    fn test_numbers_still_compare_numerically() {
        let item = json!({ "age": 20 });
        assert!(cond("age", FilterOp::Gt, json!(18)).apply(&item).unwrap());
        assert!(!cond("age", FilterOp::Gt, json!(20)).apply(&item).unwrap());
        assert!(cond("age", FilterOp::Gte, json!(20)).apply(&item).unwrap());
        // Lexicographic order would call "20" less than "9"; numeric must not.
        assert!(cond("age", FilterOp::Gt, json!(9)).apply(&item).unwrap());
    }

    /// Kinds that cannot be ordered, and mismatched pairs, do not match.
    #[test]
    fn test_unorderable_and_mismatched_kinds_do_not_match() {
        for (item, value) in [
            (json!({ "f": "2020" }), json!(2020)),
            (json!({ "f": 2020 }), json!("2020")),
            (json!({ "f": true }), json!(false)),
            (json!({ "f": [1, 2] }), json!([1])),
            (json!({ "f": null }), json!(null)),
        ] {
            for op in [FilterOp::Gt, FilterOp::Gte, FilterOp::Lt, FilterOp::Lte] {
                assert!(
                    !cond("f", op.clone(), value.clone()).apply(&item).unwrap(),
                    "{:?} {} {:?} should not match",
                    item,
                    op,
                    value
                );
            }
        }
    }

    /// A missing field never matches an ordering operator.
    #[test]
    fn test_absent_field_does_not_match() {
        let item = json!({ "other": "x" });
        assert!(!cond("f", FilterOp::Gte, json!("a")).apply(&item).unwrap());
    }

    /// Two strings are a supported comparison now, so they must not be
    /// reported as a type mismatch. Warning about a comparison the evaluator
    /// handles correctly would be noise.
    #[test]
    fn test_string_operands_are_not_a_type_mismatch() {
        for op in [FilterOp::Gt, FilterOp::Gte, FilterOp::Lt, FilterOp::Lte] {
            assert!(
                is_type_compatible(&op, &json!("2020-01-01"), &json!("2021-01-01")),
                "two strings should be compatible under {}",
                op
            );
            assert!(
                !is_type_compatible(&op, &json!("2020"), &json!(2020)),
                "mixed kinds should still be reported under {}",
                op
            );
        }
    }
}
