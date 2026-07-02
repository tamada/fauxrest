---
title: "⚓️ Install"
date: "2026-06-30"
---

## Prerequisites

- Rust toolchain (`cargo`) installed.

## Build From Source

```bash
git clone https://github.com/username/fauxrest.git
cd fauxrest
cargo build --release
```

The binary is generated at `target/release/fauxrest`.

## Quick Start

1. Create a `data/` directory with JSON files.
2. Prepare a config file (for example `fauxrest.json`) if you need explicit serializer/layout settings.
3. Build outputs:

```bash
fauxrest build ./data fauxrest.json
```

4. Check generated files under the configured destination path.

## Example Input

```json
{
	"name": "Alice",
	"role": "Developer"
}
```
