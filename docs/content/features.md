---
title: "⭐️ Features"
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

`$private: true` suppresses generation of the target endpoint and all descendants.

### Aggregation

`$aggregate` bundles multiple endpoints or collections into one endpoint payload.

### Template Expansion

Template keys such as `${year}` can be expanded by:

- `$values` for static lists.
- `$derive` for values extracted from loaded source data.

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
