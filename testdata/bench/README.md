# Regex caching benchmark (end to end)

Measures the cost of compiling `$filter` regex patterns, which used to happen
once per record instead of once per pattern (issue #4).

This is the whole-CLI measurement: it includes reading the dataset and writing
the output, so it reflects what a user actually waits for. `benches/filter_regex.rs`
covers the same change as a criterion microbenchmark over the evaluation loop
alone — run that with `cargo bench` when you want a repeatable regression
signal, and this one when you want the wall-clock figure.

## What it isolates

`_config.json` has four `regeq`/`regneq` conditions on one node, so a run over
the generated dataset performs `records x 4` pattern evaluations.

Two details keep the measurement about regex work rather than disk:

- `"$emit": []` on the `items` collection suppresses its per-record endpoints.
  Without it the run writes one file per record and file I/O dominates
  completely.
- The `matched` node emits only `list`, so the whole run produces two files
  regardless of the record count.

## Running it

```sh
python3 testdata/bench/generate.py            # 200000 records, ~10 MiB
cargo build --release
./target/release/fauxrest testdata/bench/data \
    --config testdata/bench/_config.json \
    --dest /tmp/fauxrest-bench
```

Or `just bench`, which does all three.

Pass a record count to generate a smaller set, e.g.
`python3 testdata/bench/generate.py 20000`.

The generated `data/` directory is gitignored — it is reproducible from the
script, and committing 10 MiB of synthetic JSON is not worth the repository
size.

## Measured

200000 records x 4 conditions = 800000 pattern evaluations, release build,
five runs each, on an Apple Silicon macOS machine:

| | runs (s) | median |
|---|---|---|
| compiling per record | 7.60, 6.79, 6.33, 6.19, 6.26 | 6.33 s |
| compiling once per pattern | 0.50, 0.44, 0.48, 0.47, 0.45 | 0.47 s |

About 13x faster. The gap widens with more serializers, since each one
re-materializes the routing tree and so re-evaluates every condition.

`cargo bench` reports a much larger multiple (roughly 175x) for the same
change, because it times only the evaluation loop. The difference between the
two numbers is the JSON parsing and file writing that a real run cannot avoid.
