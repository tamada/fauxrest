//! Benchmarks the `$filter` evaluation loop, the path issue #4 was about:
//! regex patterns used to be compiled once per record instead of once per
//! pattern.
//!
//! `testdata/bench` measures the same change end to end through the CLI, which
//! is what a user actually waits for. This benchmark isolates the evaluation
//! loop instead, so a regression shows up without the noise of JSON parsing and
//! file writes, and without needing a generated dataset.
//!
//! Note that the compiled-pattern cache is process-wide, so criterion's warmup
//! leaves it hot: what is timed here is matching, not compiling. That is
//! exactly the regression signal wanted — dropping the cache puts compilation
//! back inside the timed loop and the numbers jump by an order of magnitude.
//!
//! The `eq` variant runs the same record count through the same number of
//! conditions using a trivial operator, as a floor for what the loop costs
//! without any regex work at all.
//!
//! Measured on an Apple Silicon macOS machine: 2.3 ms with the cache versus
//! 405 ms without it, roughly 175x. The `eq` baseline stays at 0.6-0.8 ms
//! either way, which is what confirms the difference belongs to the regex path
//! rather than the loop around it. The end-to-end figure in
//! `testdata/bench/README.md` is a smaller multiple (about 13x) because a CLI
//! run also parses megabytes of JSON and writes its output.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use fauxrest::filter::{FilterCondition, FilterOp};
use serde_json::{Value, json};

const RECORD_COUNT: usize = 10_000;
const TAGS: [&str; 3] = ["alpha", "beta", "gamma"];

/// Builds records with the same shape as `testdata/bench/generate.py`.
fn records(count: usize) -> Vec<Value> {
    (0..count)
        .map(|i| {
            json!({
                "id": i,
                "name": format!("item-{:06}", i),
                "tag": TAGS[i % TAGS.len()],
            })
        })
        .collect()
}

fn condition(field: &str, op: FilterOp, value: Value) -> FilterCondition {
    FilterCondition {
        field: field.to_string(),
        op,
        value,
    }
}

/// The four regex conditions from `testdata/bench/_config.json`.
fn regex_conditions() -> Vec<FilterCondition> {
    vec![
        condition("name", FilterOp::RegEq, json!("^item-0[0-9]{5}$")),
        condition("tag", FilterOp::RegNeq, json!("^gamma$")),
        condition("name", FilterOp::RegEq, json!("item")),
        condition("tag", FilterOp::RegEq, json!("^(alpha|beta)$")),
    ]
}

/// Four conditions that need no pattern matching, as a baseline.
fn eq_conditions() -> Vec<FilterCondition> {
    vec![
        condition("tag", FilterOp::Neq, json!("gamma")),
        condition("tag", FilterOp::Neq, json!("delta")),
        condition("name", FilterOp::Neq, json!("")),
        condition("id", FilterOp::Exists, json!(true)),
    ]
}

/// Counts the records satisfying every condition, mirroring what
/// `apply_filters` does for a collection endpoint.
fn count_matching(items: &[Value], conditions: &[FilterCondition]) -> usize {
    items
        .iter()
        .filter(|item| {
            conditions
                .iter()
                .all(|cond| cond.apply(item).expect("conditions are valid"))
        })
        .count()
}

fn bench_filters(c: &mut Criterion) {
    let items = records(RECORD_COUNT);
    let regex_conds = regex_conditions();
    let eq_conds = eq_conditions();

    let mut group = c.benchmark_group("filter");
    group.bench_function("four_regex_conditions_over_10k_records", |b| {
        b.iter(|| black_box(count_matching(&items, &regex_conds)))
    });
    group.bench_function("four_eq_conditions_over_10k_records", |b| {
        b.iter(|| black_box(count_matching(&items, &eq_conds)))
    });
    group.finish();
}

criterion_group!(benches, bench_filters);
criterion_main!(benches);
