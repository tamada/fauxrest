
git_revision := `git rev-parse --short HEAD`
app_version := `awk -F'"' '/^\[package\]/{p=1} p && /^version *=/{print $2; exit}' Cargo.toml`
build_date := `date -u +%Y-%m-%dT%H:%M:%SZ`

image_quay   := 'quay.io/tama5'
image_github := 'ghcr.io/tamada'
container_image := image_quay

container_runner := "docker"

test:
    cargo llvm-cov

generate_demo: build
    PATH=target/debug vhs .github/assets/demo.tape

# Criterion microbenchmark of the $filter evaluation loop.
bench-micro:
    cargo bench --bench filter_regex

# Times $filter regex evaluation end to end over a generated dataset.
# See testdata/bench/README.md. Pass a record count as `just bench 20000`.
bench count="200000":
    python3 testdata/bench/generate.py {{count}}
    cargo build --release
    rm -rf /tmp/fauxrest-bench
    time ./target/release/fauxrest testdata/bench/data \
        --config testdata/bench/_config.json \
        --dest /tmp/fauxrest-bench

docs:
    cargo llvm-cov --html
    cd docs && {{ container_runner }} run -it --rm hugomods/hugo:0.163.0
    rm -rf docs/public/coverage && cp -r target/llvm-cov/html docs/public/coverage

build: test
    cargo build --release

container-local:
    docker build \
        --build-arg GIT_REVISION={{git_revision}} \
        --build-arg BUILD_DATE={{build_date}} \
        --build-arg VERSION={{app_version}} \
        -t {{container_image}}/fauxrest:latest \
        -t {{container_image}}/fauxrest:{{ app_version }} \
        -f Containerfile \
        .

container:
    docker buildx build --push \
        --platform linux/amd64,linux/arm64 \
        --build-arg GIT_REVISION={{git_revision}} \
        --build-arg BUILD_DATE={{build_date}} \
        --build-arg VERSION={{ app_version }} \
        -t {{container_image}}/fauxrest:latest \
        -t {{container_image}}/fauxrest:{{ app_version }} \
        -f Containerfile \
        .

# Pre-push checks: clippy, format, and tests
pre-push:
    #!/bin/bash
    set -e
    echo "=== Running pre-push checks ==="

    echo "Running clippy..."
    cargo clippy -- -D warnings

    echo "Checking format..."
    if cargo fmt --all --check > /dev/null 2>&1; then
        echo "✓ Format OK"
    else
        echo "❌ Format issues detected. Run 'just fmt' to fix."
        exit 1
    fi

    echo "Running tests..."
    cargo test

    echo "✓ All checks passed, ready to push"

# Apply `cargo fmt` across the workspace. Commit it alone with a `style:` subject.
fmt:
    cargo fmt --all

# Record HEAD in .git-blame-ignore-revs. Run *after* committing a formatting pass.
fmt-record:
    #!/usr/bin/env bash
    set -euo pipefail

    hash=$(git rev-parse HEAD)
    subject=$(git log -1 --pretty=%s)

    # Idempotent: checked first, so re-running after this recipe dirties the
    # tree with its own edit still reports cleanly instead of tripping the
    # dirty-tree guard below.
    if grep -qx "$hash" .git-blame-ignore-revs 2>/dev/null; then
        echo "already recorded: $hash ($subject)"
        exit 0
    fi

    if [ "${subject#style:}" = "$subject" ]; then
        echo "⚠ HEAD is not a formatting commit: $subject" >&2
        echo "  Only 'style:' revisions belong in .git-blame-ignore-revs." >&2
        exit 1
    fi

    # Ignore this file itself: it is the one thing this recipe is allowed to change.
    if ! git diff --quiet HEAD -- . ':!.git-blame-ignore-revs' ; then
        echo "⚠ Working tree is dirty; commit the formatting pass before recording." >&2
        exit 1
    fi

    # Entries must be bare 40-character hashes on their own line; git rejects
    # abbreviated names and does not accept trailing comments.
    printf '\n# %s\n%s\n' "$subject" "$hash" >> .git-blame-ignore-revs
    echo "✓ recorded $hash ($subject)"
