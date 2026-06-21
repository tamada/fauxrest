# prest Development Guidelines and Coding Standards

This document defines the strict architectural boundaries, coding standards, and testing criteria to be followed by developers contributing to or maintaining `prest`.

---

## 1. Crate Architecture

To maximize decoupling and reusability, `prest` clearly separates library logic from binary orchestration:

- **Library Crate (`prest`):** Implemented in `src/lib.rs`. Encapsulates core business logic, serializers, layouts, and compilation logic. It must be free of CLI parser dependencies (`clap`, etc.) and expose a clean, programmatic interface.
- **Binary Crate (`prest-bin`):** Implemented in `src/cmd/main.rs`. Responsible for CLI argument parsing, validation, and command dispatching.
- **Dependency Flow Constraint:** `src/cmd/` depends on `prest` (library). The library must **never** reference any code within `src/cmd/` or depend on CLI state.

---

## 2. Mandatory Coding Standards

### The 10-Line Rule
To ensure clarity, testability, and modularity, **no function, method, helper, or test function may exceed 10 lines of code**. If a function risks exceeding this, it must be refactored into smaller, single-responsibility helper functions.

### Early Returns (Flat Code)
Deeply nested `if-else` blocks are prohibited. Utilize guard clauses to maintain flat indentation and ensure control flow remains intuitive and readable.

### Documentation Requirements
- **All functions/methods** (including private ones) must include concise comments explaining their purpose.
- **All public functions/methods** must use Rustdoc standard format (`///`) and include fully runnable examples (`/// # Examples`) to verify behavior via doctests.

---

## 3. Quality Assurance & Hermetic Testing

### Doctests
Code examples in public API docblocks are executed automatically. Added APIs must include working doctests.

### Integration Tests (`tests/`)
Key data transformations, edge cases, and CLI routing modes are validated as black-box integration tests in the `tests/` directory.

### Build & Test Standards
All pull requests must meet the following:
1. Zero compiler warnings and zero linter errors.
2. `cargo test` passes entirely.

### Hermeticity (Isolated Testing)
Tests must be completely deterministic, running flawlessly in offline environments without external network dependencies. Temporary files and directories generated during testing must be automatically cleaned up using RAII tools (e.g., `tempfile`).

---

## 4. Version Control Workflow (Jujutsu)

This project uses the **Jujutsu (`jj`)** version control system.

### Workflow Procedures
1. **Create a bookmark**: `jj bookmark create feat-my-feature`
2. **Automatic tracking**: Jujutsu automatically tracks working copy changes; manual `git add`/`commit` is unnecessary.
3. **Set Revision Description**: Provide a clear, detailed commit message.
   ```bash
   jj describe -m "feat: add xxx transformation

   - Enhance automatic inference from data/ json structure
   - Implement transformation logic in src/lib.rs
   - Add unit/integration tests"
   ```
4. **Run tests**: `cargo test`
5. **Check status**: `jj status`

### Commit Rules
- **Atomic Commits**: Each revision (commit) must contain exactly one logical change. Never mix bug fixes, documentation updates, and core logic modifications.
- **Detailed Description**: Provide thorough explanations of the "WHY" (rationale), impact, and technical approach in multi-line commit messages. One-line messages are not accepted.
