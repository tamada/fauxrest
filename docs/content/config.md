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
`-l/--layout`, `--minify`, `--no-minify`, or `--overwrite` explicitly, that value
overrides the corresponding field of every serializer entry defined here. Options
you omit keep the values from this file.

## Static File Copying

By default, non-JSON files in the input data directory (images, CSS, fonts, …)
are ignored. The top-level `$static` key opts them into being copied verbatim
into every serializer `dest`, preserving sub-directory structure.

Two shapes are accepted:

Shorthand (include globs only):

```json
{
	"$static": ["*.png", "css/**"]
}
```

Full form with explicit allow/deny lists:

```json
{
	"$static": {
		"include": ["*.png", "css/**"],
		"exclude": ["**/*.secret.png", "private/**"]
	}
}
```

- `include`: glob patterns that **allow** a static file to be copied.
- `exclude`: glob patterns that **deny** a static file from being copied.

Globs are matched against each file's path relative to the data directory
(using `/` separators). Invalid glob patterns are rejected at load time with a
configuration error.

### Priority

- **Deny by default.** Without an `include` glob (or the `--copy-static` command
  line flag), nothing is copied.
- **`exclude` (deny) always wins.** A file matching an `exclude` glob is never
  copied, even when `--copy-static` forces every file to be allowed.
- Data (`.json`) files and configuration files (`_config.json`,
  `_fauxrest.json`, `.config.json`, `.fauxrest.json`) are **always excluded** —
  they are treated as inputs, never as static assets.

The `--copy-static` command line flag sets allow-all: every static file is
treated as allowed regardless of `include`, but `exclude` globs still take
precedence.

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

### Typed `$derive` values

A `$derive.pattern` extracts a regular expression capture, so the derived
value is always a string. Because `$filter` compares strictly by type, such a
value never matches a numeric field — `"2024"` does not equal `2024`. The
optional `type` key converts the derived value before it is used:

```json
{
  "papers": {
    "${year}": {
      "$derive": { "field": "year", "pattern": "^(\\d{4})", "type": "int" },
      "$filter": [{ "field": "year", "op": "eq", "value": "{year}" }]
    }
  }
}
```

Supported types are `string` and `int`. Omitting `type` performs no
conversion, which is how `$derive` behaved before 0.0.3.

`type` was introduced in 0.0.3, which also accepted `float`, `bool` and
`auto`. Those three are rejected from 0.0.4 onwards; see the release notes.

- Conversion applies to the whole derived value, so it also decides the
  generated path segment: `"type": "int"` on `"007"` produces `/7`.
- Values that cannot be converted are skipped and reported as non-derivable
  rather than failing the build.

The set is deliberately small: a derived value becomes a path segment, and
`string` and `int` are the kinds that makes sense for. `type` is an additive
enum, so more can be introduced later without invalidating a configuration
that works today.
