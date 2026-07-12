---
title: "⚙️  Configuration"
date: "2026-06-30"
---

## Configuration Tiers

`fauxrest` supports three tiers depending on required control:

1. Zero-config: infer endpoints from `./data` structure.
2. Convention overlay: patch inferred tree via `_config.json` style files.

Auto-discovery for config files checks:

- `_config.json`
- `_fauxrest.json`
- `.config.json`
- `.fauxrest.json`

## Serializer Configuration

Set serializers under `$config.serializers`.

```json
{
	"$config": {
		"serializers": [
			{ "serializer": "json", "layout": "index", "dest": "./dist/api" },
			{ "serializer": "typescript", "layout": "file", "dest": "./dist/modules" },
			{ "serializer": "sqlite", "dest": "./dist/db/api.db" }
		]
	}
}
```

Supported serializers:

- `json`
- `typescript` (or `javascript`, `js`, `ts`)
- `sqlite`

`minify` is configurable per serializer.

`overwrite` is configurable per serializer (defaults to `false`). When `false`
and the serializer `dest` directory already contains files, the build aborts
with an error instead of clobbering the existing output. Set it to `true`
(or pass `--overwrite` on the command line) to allow overwriting.

Command-line options take precedence over the configuration file
(**CLI > config > default**). When you pass `-d/--dest`, `-s/--serializer`,
`-l/--layout`, `--minify`, or `--overwrite` explicitly, that value overrides the
corresponding field of every serializer entry defined here. Options you omit keep
the values from this file.


## Layout Configuration

Supported layouts:

- `index`: emits `/path/index.[ext]`
- `file`: emits extensionless files when safe
- `extension`: emits `/path.[ext]`

In `file` layout, smart fallback avoids file-directory collisions by emitting
`index.[ext]` when a path also needs child paths.

## Overlay Directives

In overlay config, keys starting with `$` are directives.

- `$emit`: emit endpoint at a path.
- `$filter`: filter collection records.
- `$pick`: allowlist keys.
- `$omit`: denylist keys.
- `$aggregate`: merge sources into one endpoint.

Template sub-paths like `${year}` support:

- `$values`: static expansion list.
- `$derive`: expansion derived from data.

`$values` and `$derive` are mutually exclusive at the same template node.
