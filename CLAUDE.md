# fauxrest Development Guide

## Quick Start

### Prerequisites
- Rust (latest stable)
- Just (task runner)
- Docker (for container builds)

### Setup

```bash
# Clone the repository
git clone https://github.com/tamada/fauxrest.git
cd fauxrest

# Optional: Enable pre-push quality checks
# (See "Git Hooks" section below)
git config core.hooksPath .githooks
```

## Development Workflow

### Build and Test

```bash
# Run all tests with coverage
just test

# Build release binary
just build

# Run quality checks before pushing (optional, see setup above)
just pre-push
```

### Code Quality

We use `cargo clippy`, `cargo fmt`, and `cargo test` to maintain code quality.

```bash
# Check formatting issues
cargo fmt --all --check

# Apply formatting and update git-blame-ignore-revs
just fmt-fix

# Run clippy lints
cargo clippy -- -D warnings
```

## Git Hooks (Optional)

### Enable Pre-Push Checks

To automatically run quality checks before pushing, enable the pre-push hook:

```bash
git config core.hooksPath .githooks
```

This will run:
1. `cargo clippy -- -D warnings` - Check for lint warnings
2. `cargo fmt --all --check` - Verify code formatting
3. `cargo test` - Run all tests

If any check fails, the push is blocked. To fix formatting issues:

```bash
just fmt-fix
```

### Disable Pre-Push Checks

```bash
git config --unset core.hooksPath
```

## Formatting and git-blame

### Background

We maintain `.git-blame-ignore-revs` to exclude formatting-only commits from `git blame` output. This allows reviewers to see the actual code changes, not just formatting adjustments.

### When You Run `cargo fmt`

If you make formatting-only changes and create a commit, run:

```bash
just fmt-fix
```

This will:
1. Apply `cargo fmt --all`
2. Automatically record the commit hash in `.git-blame-ignore-revs`

### Using git blame with ignored revisions

```bash
# Ignore formatting commits when checking blame history
git blame --ignore-revs-file .git-blame-ignore-revs src/main.rs
```

Or configure it globally:

```bash
git config blame.ignoreRevsFile .git-blame-ignore-revs
```

## CI/CD

The GitHub Actions workflow in `.github/workflows/build.yaml` runs on every push and pull request:

- `cargo clippy -- -D warnings`
- `cargo fmt --all --check`
- `cargo build --release`
- `cargo llvm-cov` (coverage on Ubuntu)

All checks must pass before merging.

## Project Structure

- `src/` - Main source code
  - `main.rs` - CLI entry point
  - `cmd/` - Command implementations
  - `config.rs` - Configuration parsing
  - `serializers.rs` - Output format handlers
  - `static_files.rs` - Static file serving
- `tests/` - Integration tests
- `.github/workflows/` - CI/CD configuration
- `.githooks/` - Optional Git hooks
- `justfile` - Development tasks

## Milestones

Current development targets v1.0.0 release.

See GitHub Issues for current development priorities.
