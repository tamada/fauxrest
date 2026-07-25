---
title: "⚓️ Install"
date: "2026-06-30"
---

## 🍺 Homebrew

```bash
brew install tamada/tap/fauxrest
```

## 📦 Build with Cargo

- You should install Rust toolchain (`cargo`).

```bash
cargo install fauxrest
```

## 💪 Build From Source

- You should install Rust toolchain (`cargo`).

```bash
git clone https://github.com/username/fauxrest.git
cd fauxrest
cargo build --release
```

The binary is generated at `target/release/fauxrest`.

## 🐳 Docker

Fauxrest can run on docker like container runner.
This example uses `docker` command, however, you can replace it to your favorit container runner such as [podman](https://podman.io), [finch](https://github.com/runfinch/finch), [apple/container](https://github.com/apple/container), and/or [WSL Container](https://learn.microsoft.com/windows/wsl/wsl-container).

```sh
docker run -it --rm -v $PWD:/opt quay.io/tama5/fauxrest:latest data
```

## Quick Start

1. Create a `data` directory with JSON files.
2. Prepare a config file (for example `fauxrest.json`) if you need explicit serializer/layout settings.
3. Build outputs:

```bash
fauxrest --config fauxrest.json data
```

4. Check generated files under the configured destination path.

## Example Input

```json
{
	"name": "Alice",
	"role": "Developer"
}
```
