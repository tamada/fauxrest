# prest Technical Specifications and Data Transformation

`prest` is a high-performance static API generator designed for static hosting environments. It includes advanced build specifications, transformation logic, and developer experience (DX) features.

---

## 1. Configuration Tiers

`prest` supports three configuration tiers depending on the development phase and flexibility required:

- **Tier 1: Zero-Config**: If no configuration file exists, `prest` automatically builds an API from the structure in `./data` using the default serializer (`json`) and delivery layout (`index`).
- **Tier 2: Simple Routing (`routes.yml`)**: Allows route customization, path overrides, and partial data extraction via JSONPath.
- **Tier 3: Programmatic Configuration (`api.config.js`)**: Allows writing complex data transformation functions using JavaScript/TypeScript.

---

## 2. Core REST Transformation Patterns

When using custom routing, `prest` processes input data using four structural transformation patterns:

1. **Single Object (`type: single`)**: Copies a target JSON object directly to the path.
2. **Collection (`type: list`)**: Compiles a JSON array directly as a list endpoint.
3. **Item Detail (`type: detail`)**: Traverses an array, generating individual physical files for each item based on dynamic segments (e.g., `:id`).
4. **Grouping (`type: group`)**: Groups objects by a specified field and outputs separate resource lists for each group.

---

## 3. Serializer and Layout Specifications

`prest`'s compilation system is defined by two orthogonal concepts: the **"Serializer"** (which determines the physical data structure) and the **"Layout"** (which determines the file system placement).

### A. Three Physical Serializers
Each serializer renders input data (JSON) into a specific physical format.

1. **`json` Serializer (Default)**
   - **Spec**: Serializes data as plain JSON text.
   - **Extension**: `.json`
2. **`typescript` (or `js`) Serializer**
   - **Spec**: Converts data into ESM (ECMAScript Modules) code (e.g., `export const data = [...]`).
   - **Extension**: `.ts` (or `.js`)
   - **Use case**: Ideal for frontend projects using SSG tools (Astro, Next.js, SvelteKit) to import data as modules during the build process.
3. **`sqlite` Serializer**
   - **Spec**: Compiles data into a portable binary relational database format (SQLite).
   - **Extension**: `.db` (or `.sqlite`)
   - **Use case**: Perfect for client-side applications (using SQL.js or WASM-SQLite) that need to perform complex relational queries locally.

### B. Three Delivery Layouts
The `layout` determines the relationship between endpoint URL resolution and physical file placement.

- **`index` Layout (Default)**: Outputs endpoints as `/endpoint/index.[ext]`. Highly compatible with all static web servers, maintaining clean URLs.
- **`file` Layout**: Outputs endpoints as extensionless files (`/endpoint`). 
  - **Smart Fallback Specification**: To avoid physical file-directory collisions, **collections that contain sub-paths (like individual resource items) are automatically replaced (fallback) by `.../index.[ext]` files during compilation.**
- **`extension` Layout**: Outputs endpoints with explicit extensions (`/endpoint.[ext]`). 100% web server compatible.

---

## 4. Data Aggregation & Bundle Pattern

"Bundling"—the process of merging multiple endpoints or an entire database into a single `/db.json` (or any single path)—is implemented via **Aggregation Settings**, not by switching engine output modes.

### Realization via Aggregation (`aggregate`)
By using the `$index: { "aggregate": [...] }` meta-key, you can configure an endpoint to merge multiple collections into a single array.

- **Example: Bundling to `/db.json` (`routes.yml`)**
  ```yaml
  "/db.json":
    $index:
      aggregate:
        - "data/users.json"
        - "data/profile.json"
        - "data/papers.json"
  ```

---

## 5. Advanced JSONPath Extraction

To extract only specific sub-trees from large monolithic datasets (e.g., local CMS outputs), you can use standard JSONPath queries directly in the `source` parameter:
```yaml
# Extracts only the 'activities' array from db.json
"/activities": "data/db.json$.activities"
```

---

## 6. Data Sanitization (`pick` & `omit`)

To prevent the leakage of sensitive data (passwords, tokens, drafts) to public CDNs, filtering can be applied during compilation:
- **`pick` (Allowlist)**: Keep only the specified keys.
- **`omit` (Denylist)**: Explicitly remove specified keys.

---

## 7. Automated CORS and Platform Headers

`prest` automatically generates server configuration files during build time to resolve CORS constraints:
- **Cloudflare Pages / Netlify**: Generates a `_headers` file with wildcard CORS rules.
- **Vercel**: Generates a `vercel.json` configuration file.

---

## 8. Automated TypeScript Type Generation (`.d.ts`)

To ensure type safety, `prest` parses JSON data structures during build time and automatically generates accurate TypeScript definition files.
- **Output**: `dist/types/` (e.g., `profile.d.ts`, `users.d.ts`).

---

## 9. Caching & Incremental Builds

To optimize build times for large datasets, `prest` incorporates differential building:
- During build, SHA-256 hashes of all input files are calculated and stored in `.static-api-cache.json`.
- Subsequent builds verify hashes, compiling only changed files.

---

## 10. Media Asset Bundling & Path Rewriting

`prest` automatically detects relative file paths in JSON values, copies the referenced assets to `/dist/api/assets/`, and rewrites the paths in the generated JSON to the public relative URL.
