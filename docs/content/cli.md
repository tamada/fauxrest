---
title: "🏃 Usage"
---

`prest` provides a small command surface for build and local verification workflows.

## How to run `prest`

### Build psuede REST data

Compile datasets into static outputs.

```bash
prest build --config prest.json ./data
```

Typical behavior:

- Reads data from `./data`.
- Loads configuration from the provided config path.
- Runs configured serializers and layouts.
- Writes artifacts to serializer-specific destinations.

### Serve

Start a local development server with rebuild support.

```bash
prest serve ./data --port 8080
```

Serve mode capabilities:

- Correct response content types.
- Local CORS headers for frontend development.
- File watching and incremental rebuilds on changes.

### Typical Local Loop

1. Edit JSON data or config.
2. Run `prest build` for deterministic artifact generation.
3. Use `prest serve` while integrating a frontend locally.

## 🐳 Container image support

[![quay.io](https://img.shields.io/badges/quay.io-quay.io/tama5/prest:latest-EE0000?logo=redhat)](https://quay.io/repository/tama5/prest)

```sh
docker run -it --rm -v $PWD:/opt quay.io/tama5/prest:latest 
```
