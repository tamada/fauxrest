# fauxrest Technical Specifications and Data Transformation

`fauxrest` is a high-performance static API generator designed for static hosting environments. It includes advanced build specifications, transformation logic, and developer experience (DX) features.

---

## 1. Configuration Tiers

`fauxrest` supports three configuration tiers depending on the development phase and flexibility required:

- **Tier 1: Zero-Config**: If no configuration file exists, `fauxrest` automatically infers and builds an API from the structure in `./data` using convention-over-configuration.
- **Tier 2: Convention Overlay (`_config.json`)**: Allows patching the automatically inferred API tree using a structured JSON configuration that mirrors the API routes. 
  - **Auto-Discovery**: `fauxrest` automatically searches for the configuration file in the following order: `_config.json`, `_fauxrest.json`, `.config.json`, `.fauxrest.json`.
- **Tier 3: Programmatic Configuration (`api.config.js`)**: Allows writing complex data transformation functions using JavaScript/TypeScript.

---

## 2. Serializer and Layout Specifications

`fauxrest`'s compilation system is defined by two orthogonal concepts: the **"Serializer"** (which determines the physical data structure) and the **"Layout"** (which determines the file system placement).

### A. Three Physical Serializers
Each serializer renders input data (JSON) into a specific physical format.

1. **`json` Serializer (Default)**
   - **Spec**: Serializes data as plain JSON text.
   - **Extension**: `.json`
  - **Option**: With `minify: true`, emits compact (non-pretty) JSON.
2. **`typescript` (or `js`) Serializer**
   - **Spec**: Converts data into ESM (ECMAScript Modules) code (e.g., `export const data = [...]`).
   - **Extension**: `.ts` (or `.js`)
   - **Use case**: Ideal for frontend projects using SSG tools to import data as modules during the build process.
  - **Option**: With `minify: true`, embeds compact JSON in the generated module.
3. **`sqlite` Serializer**
   - **Spec**: Compiles data into a portable binary relational database format (SQLite).
   - **Extension**: `.db` (or `.sqlite`)
   - **Use case**: Perfect for client-side applications that need to perform complex relational queries locally.

`minify` is configured per serializer. In multi-serializer setups, each serializer can enable or disable it independently.

### B. Three Delivery Layouts

The `layout` determines the relationship between endpoint URL resolution and physical file placement.

- **`index` Layout (Default)**: Outputs endpoints as `/endpoint/index.[ext]`. Highly compatible with all static web servers, maintaining clean URLs.
- **`file` Layout**: Outputs endpoints as extensionless files (`/endpoint`). 
  - **Smart Fallback Specification**: To avoid physical file-directory collisions, collections that contain sub-paths are automatically replaced (fallback) by `.../index.[ext]` files during compilation.
- **`extension` Layout**: Outputs endpoints with explicit extensions (`/endpoint.[ext]`). 100% web server compatible.

---

## 3. Convention Overlay Schema (Tier 2 Routing)

To adhere strictly to the "Convention over Configuration" philosophy, advanced configurations in `_config.json` use an **Overlay Schema**. Instead of explicitly defining every route, the configuration mirrors the inferred API tree. 

Keys prefixed with `$` are interpreted as **directives** modifying the data at that level. Keys without the `$` prefix define **sub-paths**.

When using template-style sub-path keys such as `"${year}"`, provide `$values` in the child node so endpoints can be expanded statically at build time.

When values are not known in advance, use `$derive` to derive expansion values from loaded input data. `$derive` is only valid under template sub-path keys and cannot be used together with `$values`.

```json
{
  "api": {
    "activities": {
      // Applies to the root /api/activities collection
      "$filter": [
        { "field": "is_public", "op": "eq", "value": true }
      ]
    },
    "job-histories": {
      // Defines a new sub-path: /api/job-histories/current
      "current": {
        "$filter": { "field": "to", "op": "eq", "value": "Present" }
      }
    },
    "activities": {
      // Statically expands to /api/activities/2026, /api/activities/2025, ...
      "${year}": {
        "$values": ["2026", "2025"],
        "$filter": [
          { "field": "from", "op": "contains", "value": "{year}" }
        ]
      }
    },
    "events": {
      "${year}": {
        "$derive": { "field": "from", "pattern": "^(\\d{4})" },
        "$filter": [
          { "field": "from", "op": "contains", "value": "{year}" }
        ]
      }
    },
    "profile": {
      "$aggregate": ["job-histories", "activities", "degrees", "skills"]
    }
  }
}
```

---

## 4. Advanced Filtering (`$filter`)

The `$filter` directive allows extracting specific items from a collection. To ensure type safety, ease of implementation, and debuggability, filters utilize a strict `field`, `op`, and `value` object structure.

### Supported Operators (`op`)
- `eq` (Equal)
- `neq` (Not Equal)
- `gt` (Greater Than)
- `gte` (Greater Than or Equal)
- `lt` (Less Than)
- `lte` (Less Than or Equal)
- `contains` (String or Array contains)
- `exists` (Field exists, `value` is boolean)
- `regeq` (Regular expression match)
- `regneq` (Regular expression does not match)

**Example:**
```json
"$filter": [
  { "field": "age", "op": "gte", "value": 18 },
  { "field": "status", "op": "eq", "value": "active" }
]
```

### Parent/Child `$filter` Precedence

If a child node defines `$filter`, the parent node's `$filter` is not inherited and the child filter **overrides** it. Parent filter inheritance only applies when the child does not define `$filter`.

---

## 5. Data Sanitization (`$pick` & `$omit`)

To prevent the leakage of sensitive data (passwords, tokens, internal notes) to public CDNs, filtering can be applied during compilation at any node in the API tree:
- **`$pick` (Allowlist)**: Keep only the specified keys.
- **`$omit` (Denylist)**: Explicitly remove specified keys.

---

## 6. Endpoint Visibility Control (`$private`)

Nodes configured with `$private: true` **do not emit the endpoint itself or its descendants**. For example, if configured on `users`, `/users` and `/users/...` are not generated and are omitted from discovery (`/index.json`).

- Use `$pick` / `$omit` for field-level data sanitization.

**Example:**
```json
{
  "users": {
    "$private": true
  }
}
```

---

## 6.1 Endpoint Emission Control (`$emit`)

For array payloads containing `id`, fauxrest normally emits item endpoints (e.g. `/users/1`).
Use `$emit` to control collection and item endpoint output.

- `$emit: ["list"]` emits collection endpoint files only.
- `$emit: ["ids"]` emits per-item endpoint files only.
- `$emit: ["list", "ids"]` emits both.
- `$emit: []` emits neither (allowed for intentional no-output nodes).

Legacy `$emit_list`, `$emit_id`, and `$emit_items` are still accepted for backward compatibility.

**Example:**
```json
{
  "users": {
    "$emit": ["list"]
  }
}
```

---

## 7. Data Aggregation & Bundle Pattern (`$aggregate`)

"Bundling" merges multiple endpoints or entire datasets into a single endpoint, ideal for client-side applications that fetch the entire database on load. 

By using the `$aggregate` directive, you can merge multiple collections into a single array at an arbitrary path.

**Example: Bundling to `/api/db`**
```json
{
  "api": {
    "db": {
      "$aggregate": ["users", "profile", "papers"]
    }
  }
}
```

---

## 8. Automated CORS and Platform Headers

`fauxrest` automatically generates server configuration files during build time to resolve CORS constraints:
- **Cloudflare Pages / Netlify**: Generates a `_headers` file with wildcard CORS rules.
- **Vercel**: Generates a `vercel.json` configuration file.

---

## 9. Automated TypeScript Type Generation (`.d.ts`)

To ensure type safety, `fauxrest` parses JSON data structures during build time and automatically generates accurate TypeScript definition files.
- **Output**: `dist/types/` (e.g., `profile.d.ts`, `users.d.ts`).

---

## 10. Caching & Incremental Builds

To optimize build times for large datasets, `fauxrest` incorporates differential building:
- During build, SHA-256 hashes of all input files are calculated and stored in `.static-api-cache.json`.
- Subsequent builds verify hashes, compiling only changed files.

---

## 11. Media Asset Bundling & Path Rewriting

`fauxrest` automatically detects relative file paths in JSON values, copies the referenced assets to `/dist/api/assets/`, and rewrites the paths in the generated JSON to the public relative URL.
