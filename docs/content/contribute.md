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

## Quality Gates

- Maintain passing doctests for public API examples.
- Maintain integration coverage for routing and transformation behavior.
- Require clean builds with no warnings or linter issues.
- Keep tests deterministic and isolated.

CI runs `cargo clippy -- -D warnings` and `cargo fmt --all --check` on every push
and pull request. To run the same gates locally before sharing a change:

```sh
just pre-push
```

An optional `pre-push` hook runs the same checks automatically. It is opt-in:

```sh
git config core.hooksPath .githooks   # enable
git config --unset core.hooksPath     # disable
```

Note that `jj` does not run Git hooks, so the hook does not fire on
`jj git push`. Run `just pre-push` directly when working through `jj`.

## Formatting

Formatting-only revisions add noise to `git blame`, so they are recorded in
`.git-blame-ignore-revs`. After running a formatting pass, use:

```sh
just fmt-fix
```

which applies `cargo fmt --all` and appends the current revision to
`.git-blame-ignore-revs` if it is not already listed.

To skip those revisions when reading history:

```sh
git config blame.ignoreRevsFile .git-blame-ignore-revs
```

## Version Control Workflow

Project workflow uses Jujutsu (`jj`):

1. Create bookmark for each task.
2. Keep revisions atomic (single logical change per revision).
3. Write detailed revision descriptions with rationale and impact.
4. Run tests before sharing or landing changes.
