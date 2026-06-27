---
title: "👩‍💻 How to contribute 🧑‍💻"
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

## Quality Gates

- Maintain passing doctests for public API examples.
- Maintain integration coverage for routing and transformation behavior.
- Require clean builds with no warnings or linter issues.
- Keep tests deterministic and isolated.

## Version Control Workflow

Project workflow uses Jujutsu (`jj`):

1. Create bookmark for each task.
2. Keep revisions atomic (single logical change per revision).
3. Write detailed revision descriptions with rationale and impact.
4. Run tests before sharing or landing changes.
