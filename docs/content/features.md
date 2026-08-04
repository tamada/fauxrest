---
title: "⭐️ Features"
date: "2026-06-30"
---

## Data Transformation

### Filtering

`$filter` supports structured conditions with `field`, `op`, and `value`.

Supported operators:

- `eq`, `neq`
- `gt`, `gte`, `lt`, `lte`
- `contains`
- `exists`
- `regeq`, `regneq`

Child `$filter` overrides parent `$filter` when both are present.

### Sanitization

- `$pick`: keep only selected fields.
- `$omit`: remove selected fields.

### Visibility Control

Two directives withhold output, and they differ in reach.

`$emit` controls endpoint emission at the node it appears on.

- `$emit: ["list"]` emits only collection endpoints.
- `$emit: ["ids"]` emits only per-item endpoints.
- `$emit: ["list", "ids"]` emits both.
- `$emit: []` emits neither.

`$emit` does not reach the node's sub-paths, and a sub-path with no `$emit` of
its own emits everything. So this publishes the whole dataset one level down,
despite the empty `$emit`:

```json
{
	"staff": {
		"$emit": [],
		"by-name": {}
	}
}
```

`$skip: true` leaves the node **and everything beneath it** ungenerated. A
skipped node is never descended into, so no sub-path can publish through it —
including one added later by someone who did not notice the `$skip`, and
including a descendant that sets `$skip: false`.

```json
{
	"staff": {
		"$skip": true,
		"by-name": {}
	}
}
```

Nothing under `/staff` is written, and none of it appears in the discovery
index. Use `$emit: []` to drop one node's own endpoints, and `$skip` when a
subtree must stay ungenerated whatever is added to it.

`$skip` is named for what it does to the build. Nothing is generated, which is
not the same as generating something and protecting it — static hosting has no
access control, and this tool offers none.

`$private`, `$emit_list`, `$emit_id` and `$emit_items` are **not** accepted.
`$private` was described in earlier documentation but never worked; `$skip` is
the directive that does what it described.

### Aggregation

`$aggregate` bundles multiple endpoints or collections into one endpoint payload.

### Template Expansion

Template keys such as `${year}` can be expanded by:

- `$values` for static lists.
- `$derive` for values extracted from loaded source data, with an optional
  `type` (`string`, `int`) so derived values can be
  compared against non-string fields.

## Build and Runtime Experience

- Incremental builds via file hash cache.
- Automatic TypeScript `.d.ts` generation.
- Automatic media asset copy and path rewrite for referenced local files.

## Platform Integration

- Automatic `_headers` generation for Cloudflare Pages and Netlify.
- Automatic `vercel.json` generation for Vercel CORS/header behavior.

## Serializer Flexibility

- JSON output for static API endpoints.
- TypeScript/JavaScript module output for frontend build pipelines.
- SQLite output for offline/local query scenarios.
- Per-serializer `minify` option.
