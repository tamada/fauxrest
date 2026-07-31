---
title: "👩‍💻 How to contribute 🧑‍💻"
date: "2026-06-30"
---

## Crate Architecture

`fauxrest` separates core logic from CLI orchestration.

- Library crate: implemented in `src/lib.rs`.
- Binary crate: implemented in `src/cmd/main.rs`.

Dependency rule:

- CLI code may depend on library code.
- Library code must not depend on CLI modules.

## Programmatic Use

Use the library crate when embedding `fauxrest` behavior in tooling or internal pipelines.
Keep command parsing and process-level concerns outside library boundaries.

## Development Standards

- Keep functions small and single-purpose.
- Favor early returns over deep nesting.
- Document public APIs with Rustdoc and runnable examples.
- Include concise comments where logic is non-obvious.
- Keep tests deterministic and isolated.

## Setup, Commands, and Workflow

Development setup, the `just` task list, quality gates, formatting rules, and the
version control workflow are documented in
[CONTRIBUTING.md](https://github.com/tamada/fauxrest/blob/main/CONTRIBUTING.md).
