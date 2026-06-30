---
title: "🏃 Usage"
date: "2026-06-30"
---

`fauxrest` provides a small command surface for build and local verification workflows.

## How to run `fauxrest`

```sh
A CLI tool for generating static JSON APIs from local data files

Usage: fauxrest [OPTIONS] <DATA_DIR>

Arguments:
  <DATA_DIR>  Path to the input data directory

Options:
  -L, --level <LEVEL>            Specify the log level [default: warn] [possible values: error, warn, info, debug, trace]
  -c, --config <CONFIG_FILE>     Path to the configuration file
  -l, --layout <LAYOUT>          Layout to use for the output [possible values: index, file, extension]
  -d, --dest <DEST_DIR>          Path to the output directory [default: dist]
  -s, --serializer <SERIALIZER>  Serializer to use for the output. [available: json, typescript, sql] [default: json]
      --minify                   If true, minify the output
  -h, --help                     Print help (see more with '--help')
  -V, --version                  Print version
```

### Build psuede REST data

Compile datasets into static outputs.

```bash
fauxrest data_dir
```

Typical behavior:

- Reads data from `./data_dir` (The json files starts with `_` and `.` are ignored).
- Loads configuration from `data_dir` folder (finds `_config.json`, `_fauxrest.json`, `.config_json`, and `.fauxrest.json` in this order).
- Runs configured serializers and layouts.
- Writes artifacts to serializer-specific destinations.

Note that if no `dest` directory is specified,
`fauxrest` will write to the `dist` directory by default with the `index` layout.
Also, config files can be located in `data_dir` and can specify the output directory, layout, and serializer.


### Typical Local Loop

1. Edit JSON data or config.
2. Run `fauxrest` for deterministic artifact generation.

## 🐳 Container image support

[![quay.io](https://img.shields.io/badges/quay.io-quay.io/tama5/fauxrest:latest-EE0000?logo=redhat)](https://quay.io/repository/tama5/fauxrest)

```sh
docker run -it --rm -v $PWD:/opt quay.io/tama5/fauxrest:latest 
```
