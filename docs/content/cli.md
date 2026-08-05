---
title: "🏃 Usage"
date: "2026-06-30"
---

`fauxrest` provides a small command surface for build and local verification workflows.

## How to run `fauxrest`

```sh
A CLI tool for generating static JSON APIs from local data files

Usage: fauxrest [OPTIONS] [DATA_DIR]

Arguments:
  [DATA_DIR]  Path to the input data directory [default: data]

Options:
  -L, --level <LEVEL>            Specify the log level [default: warn] [possible values: error, warn, info, debug, trace]
  -c, --config <CONFIG_FILE>     Path to the configuration file
  -l, --layout <LAYOUT>          Layout to use for the output [possible values: index, file, extension]
  -d, --dest <DEST_DIR>          Path to the output directory [default: dist]
  -s, --serializer <SERIALIZER>  Serializer to use for the output. [available: json, typescript, sql] [default: json]
      --bundle                   If set, write the whole API as one api.[ext] file
      --minify                   If true, minify the output
      --no-minify                If set, disable minification (overrides config)
      --overwrite                If true, overwrite existing files in the destination directory
      --copy-static              Copy all non-JSON static files from the data directory into each destination (allow all). $static exclude globs still take precedence.
      --gencomp                  Generate completion files
  -h, --help                     Print help (see more with '--help')
  -V, --version                  Print version
```

### Copying static files

By default `fauxrest` ignores every non-JSON file in the input data directory.
Static assets (images, CSS, fonts, …) can be copied verbatim into each
serializer destination — preserving sub-directory structure — in two ways:

- Configure an allow list with the `$static` config key (see the configuration
  reference).
- Pass `--copy-static` to copy **all** static files (allow all).

Copying is **deny by default**, and **`exclude` (deny) always wins**: files
matching a `$static` `exclude` glob are never copied, even when `--copy-static`
forces allow-all. Data (`.json`) files and configuration files
(`_config.json`, `_fauxrest.json`, `.config.json`, `.fauxrest.json`) are always
excluded.

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

### Option precedence

Options resolve as **CLI > config file > built-in defaults**.
When a configuration file is loaded, any option you pass explicitly on the command
line (`-d/--dest`, `-s/--serializer`, `-l/--layout`, `--minify`, `--no-minify`,
`--overwrite`) overrides the matching field of every serializer entry in that
config. Options you do not pass keep the values from the config file (or the
defaults when no config provides them). `--minify` and `--no-minify` are mutually
exclusive: use `--no-minify` to turn minification off when the config enables it.


### Typical Local Loop

1. Edit JSON data or config.
2. Run `fauxrest` for deterministic artifact generation.

## 🐳 Container image support

[![quay.io](https://img.shields.io/badges/quay.io-quay.io/tama5/fauxrest:latest-EE0000?logo=redhat)](https://quay.io/repository/tama5/fauxrest)

```sh
docker run -it --rm -v $PWD:/opt quay.io/tama5/fauxrest:latest 
```
