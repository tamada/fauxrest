# Contributing to fauxrest

Thank you for your interest in contributing.

This project inherits the organization-wide
[Contributing Guidelines](https://github.com/tamada/.github/blob/main/CONTRIBUTING.md)
and [Code of Conduct](https://github.com/tamada/.github/blob/main/CODE_OF_CONDUCT.md)
(a Japanese translation of both is available under
[`ja/`](https://github.com/tamada/.github/tree/main/ja)). Read those first — they
cover issue reporting, pull request expectations, and the
[Conventional Commits](https://www.conventionalcommits.org/en/) message format.

This document covers only what is specific to `fauxrest`.

## Development Setup

Requirements:

- [Rust](https://www.rust-lang.org/) stable (the crate uses edition 2024)
- [just](https://github.com/casey/just) — task runner for every command below
- A container runtime (`docker`, `podman`, `finch`, …) — only for `just docs` and
  the `container*` recipes

```sh
git clone https://github.com/tamada/fauxrest.git
cd fauxrest
just test
```

## Common Tasks

| Command | Description |
|---|---|
| `just test` | Run the test suite under `cargo llvm-cov` |
| `just build` | Run tests, then build the release binary |
| `just pre-push` | Run every gate CI enforces (see below) |
| `just fmt` | Apply `cargo fmt --all` |
| `just fmt-record` | Record the committed formatting revision for `git blame` |
| `just docs` | Build the Hugo site with coverage output |
| `just bench` | Time `$filter` evaluation end to end over a generated dataset |
| `just bench-micro` | Criterion microbenchmark of the `$filter` loop |

## Quality Gates

CI runs the following on every push and pull request:

```sh
cargo clippy -- -D warnings
cargo fmt --all --check
cargo build --release
```

Coverage is additionally reported to Coveralls from the Ubuntu job.

Run the same gates locally before sharing a change:

```sh
just pre-push
```

Beyond the automated gates, please:

- keep tests deterministic and isolated;
- maintain passing doctests for public API examples;
- add integration coverage for routing and transformation behavior.

### Optional pre-push hook

The same checks are available as an opt-in Git hook:

```sh
git config core.hooksPath .githooks   # enable
git config --unset core.hooksPath     # disable
```

Note that `jj` does not run Git hooks, so this hook does **not** fire on
`jj git push`. Run `just pre-push` directly when working through `jj`.

## Formatting

Formatting-only revisions add noise to `git blame`, so they are recorded in
[`.git-blame-ignore-revs`](.git-blame-ignore-revs). Keep such revisions separate
from behavioral changes and label them `style:`:

```sh
just fmt           # apply cargo fmt --all
git commit -am "style: ..."
just fmt-record    # append the resulting revision to .git-blame-ignore-revs
```

`fmt-record` must run *after* the commit, since it records `HEAD`. Entries in that
file must be full 40-character hashes on their own line — Git rejects abbreviated
names and does not accept trailing comments.

To skip those revisions when reading history:

```sh
git config blame.ignoreRevsFile .git-blame-ignore-revs
```

## Version Control Workflow

This project is developed with [Jujutsu](https://jj-vcs.github.io/jj/) (`jj`) over a
Git backend, so contributions are equally welcome through plain Git.

1. Create a bookmark (or branch) for each task.
2. Keep revisions atomic — one logical change per revision.
3. Write descriptions that explain rationale and impact, not just the diff.
4. Run `just pre-push` before sharing or landing changes.

## Architecture Constraints

`fauxrest` separates core logic from CLI orchestration:

- Library crate: `src/lib.rs`
- Binary crate: `src/cmd/main.rs`

CLI code may depend on library code. **Library code must not depend on CLI modules.**

See [How to contribute](https://tamada.github.io/fauxrest/contribute/) on the
documentation site for the fuller architecture and design notes.

## License

By contributing, you agree that your contributions will be licensed under the
[MIT License](LICENSE), the same license as the project.
