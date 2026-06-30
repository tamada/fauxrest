# Contributing Guidelines

Thank you for your interest in contributing to `prest`! To maintain exceptional codebase health, readability, and long-term maintainability, this project enforces strict architectural boundaries and coding standards.

---

## 🏗️ Crate Architecture

To ensure clean design and programmatic reusability, `prest` is separated into library and binary crates within a single codebase:

*   **Library Crate (`prest`):** Everything inside `src/lib.rs`. This encapsulates all core data transformations, parsing, compilation, and file writer logic. It has **zero dependencies** on CLI parsers like `clap` and exposes clean programmatic interfaces.
*   **Binary Crate (`prest-bin`):** `src/main.rs` and the `src/cmd/` module. This handles CLI input parsing, argument validation, and subcommands.
*   **Dependency flow constraint:** `src/cmd/` must depend on the `prest` library. The library must **never** reference any code inside `src/cmd/` or depend on CLI state.

---

## 📏 Mandatory Coding Standards

### The 10-Line Rule
To ensure optimal modularity, testability, and clarity, **no function or method may exceed 10 lines of code** (including helper functions and tests).

*   If any function or method is growing beyond 10 lines, it **must** be broken down into smaller, highly focused helper functions.
*   This rule promotes single-responsibility functions that are easy to reason about, lint, and test.

### Documentation Requirements
*   **Every single function or method** (including internal/private ones) must include clear comments explaining its purpose.
*   **Every public function or method** must contain Rustdoc standard documentation (`///`) and a `/// # Examples` section containing fully runnable code examples (doctests).

---

## 🧪 Testing Guidelines

We prioritize robust automated testing to verify core functionality and prevent regressions.

### Doctests
Documentation examples in your public API docblocks are executed automatically as part of the test suite. Ensure any added or updated API has a working doctest.

### Integration Tests (`tests/`)
All key transformations, edge cases, and CLI routing modes are tested as black-box integration tests under the `tests/` directory.

To run the full test suite, execute:
```bash
cargo test
```
All pull requests must compile with zero compiler warnings and pass all 12+ tests.

---

## 🔄 Development and VCS Workflow (Jujutsu)

This project uses **Jujutsu (`jj`)** for version control.

### Branch Management
Instead of traditional Git branches, use Jujutsu bookmarks:

1.  Create a bookmark for your feature or bug fix:
    ```bash
    jj bookmark create my-feature-name
    ```
2.  Work on your changes. Your working copy is automatically tracked.
3.  Describe your revision to set a clear, concise commit message:
    ```bash
    jj describe -m "feat: my descriptive feature message"
    ```
4.  Run tests before wrapping up:
    ```bash
    cargo test
    ```
5.  To inspect files and modifications, run:
    ```bash
    jj status
    ```
