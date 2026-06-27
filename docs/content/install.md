---
title: "⚓️ Install"
---

## Prerequisites

- Rust toolchain (`cargo`) installed.

## Build From Source

```bash
git clone https://github.com/username/prest.git
cd prest
cargo build --release
```

The binary is generated at `target/release/prest`.

## Quick Start

1. Create a `data/` directory with JSON files.
2. Prepare a config file (for example `prest.json`) if you need explicit serializer/layout settings.
3. Build outputs:

```bash
prest build ./data prest.json
```

4. Check generated files under the configured destination path.

## Example Input

```json
{
	"name": "Alice",
	"role": "Developer"
}
```
