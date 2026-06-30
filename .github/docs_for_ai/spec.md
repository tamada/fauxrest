# Static API Generator Specification

This document details the architectural design and functional specification for a generalized **Static API Generator** (or "Pseudo-REST API Generator"). 

The tool compiles arbitrary JSON datasets into structured directories of static JSON files, providing a read-only REST API deployable directly onto cost-effective, infinitely scalable CDNs (e.g., GitHub Pages, Cloudflare Pages, Netlify, AWS S3).

---

## 1. Core Architecture & Configurations

Because input JSON shapes and output paths are unknown in advance, the tool is built around three progressive user-experience tiers:

### Tier 1: Convention over Configuration (Zero-Config Mode)
If run without any configuration file (`static-api-gen --data ./data`), the tool infers endpoints dynamically:
*   **Object files (e.g., `profile.json`):** Mapped to `/profile`.
*   **Array files (e.g., `users.json`):** Mapped to `/users` (list view), and if array objects contain an `id` field, individual items are mapped to `/users/:id` (detail view).

### Tier 2: Shorthand Routing (Simple YAML)
If basic path modifications are needed, users can define routes with simple key-value pairs in a `routes.yml` file:
```yaml
# Simplified mapping
"/users/:id": "data/users.json"
```

### Tier 3: Configuration-via-Code (JavaScript/TypeScript Config)
For highly complex data transformation, users can write standard JS/TS files (`api.config.js`) instead of learning a proprietary DSL. This allows them to use native array manipulation functions directly in the router:
```javascript
module.exports = {
  routes: {
    '/profile': 'data/profile.json',
    '/job-histories/current': () => {
      const jobs = require('./data/job-histories.json');
      return jobs.find(job => job.is_current) || null;
    }
  }
};
```

---

## 2. Endpoint Routing Patterns & Data Rules

When configuring custom routing, the tool supports four core REST transformation patterns:

1.  **Single Object (`type: single`):** Directly copies a JSON object to a designated path.
2.  **Collection (`type: list`):** Outputs an array of objects to an endpoint.
3.  **Item Detail (`type: detail`):** Loops over an array and outputs a dedicated file for each item, substituting a dynamic segment (e.g., `:id`) with a specified key.
4.  **Grouping (`type: group`):** Groups array objects by a specific field value and outputs separate category arrays (e.g., `/users/role/admin`).

### Advanced Node Targeting & Sanitization
*   **JSONPath Extraction:** Allows the `source` parameter to target nested data structures (e.g., `source: "data/db.json$.activities"`).
*   **Sanitization (`pick` & `omit`):** Security and optimization rules to clean up endpoints before export.
    *   `pick`: Only include specified keys.
    *   `omit`: Strip specified sensitive or internal keys (e.g., `password_hash`, `draft`).

---

## 3. Production & Developer Experience (DX) Features

### 3.1. Flexible Output Formats (`--format` Option)
Static web hosts resolve directory indices, extensionless files, and trailing slashes differently, which often causes runtime `404` or redirect errors. The tool provides a `--format` option to support these different environments:
*   `--format index` (Default): Outputs `/profile/index.json` so servers automatically resolve `/profile` and `/profile/`.
*   `--format file`: Outputs `/profile` as an extensionless file.
*   `--format extension`: Outputs `/profile.json` (100% compatible across all static hosts, avoiding trailing slash issues).
*   `--format bundle`: Compiles everything into a single, unified `/db.json` file for client-side loading.

### 3.2. Asset Bundling & Path Rewriting
Static data often references local media files (e.g., `"avatar": "./images/avatar.png"`).
*   The tool scans compiled JSON values for relative file paths pointing to assets, copies them to `/dist/api/assets/`, and rewrites the path in the output JSON to the final relative URL (e.g., `"avatar": "/api/assets/avatar.png"`), preventing broken image links on the client.

### 3.3. Out-of-the-Box CORS Support
To prevent browsers from blocking API requests when hosted on a different domain, the generator enables CORS out-of-the-box:
*   Generates platform-specific header files (e.g., `_headers` for Cloudflare/Netlify, `vercel.json` for Vercel) specifying `Access-Control-Allow-Origin: *`.
*   The local dev server responds with appropriate CORS headers.

### 3.4. TypeScript Type Generation (`.d.ts`)
The generator parses input JSON data structures during build time and automatically outputs accurate `.d.ts` type definitions to `dist/types/`. This gives front-end developers immediate type safety and auto-complete out of the box.

### 3.5. Internal Dev Server & Incremental Builds
*   **Local Dev Server (`static-api-gen dev`):** A lightweight server that serves files with proper `Content-Type: application/json` headers, supports CORS, and live-reloads on file changes.
*   **Incremental Builds (Caching):** Stores hashes of data sources in `.static-api-cache.json` and only rebuilds files that have changed, drastically reducing CI/CD build and local development times.

---

## 4. Non-Functional Scope: What We Won't Build & Why

To maintain the tool's core value proposition—**extreme simplicity, zero cost, and near-zero server maintenance**—the following features are explicitly designated as out-of-scope.

### 4.1. Dynamic Write Operations (`POST`, `PUT`, `DELETE`)
*   **What it is:** Allowing client-side applications to alter, create, or delete records dynamically.
*   **Why it's Out of Scope:** This is a strictly read-only generator. Supporting write operations requires stateful backends, databases, and session handlers, defeating the entire purpose of a static, zero-server API.

### 4.2. API Versioning (e.g., simultaneous `/v1` and `/v2` branches)
*   **What it is:** Maintaining multiple simultaneous schema structures on the same host to support legacy clients.
*   **Why it's Out of Scope:** Implementing backward compatibility layers natively within a static compiler introduces immense structural overhead. If an application's lifecycle has progressed to a stage where robust, concurrent API version contracts must be maintained, the project should migrate to a traditional, dynamic REST backend (e.g., FastAPI, Express) rather than trying to patch a static file system.

### 4.3. Advanced Mock Server Simulations (Latency, Status Code Mocking)
*   **What it is:** Letting developers simulate high latency (`?__delay=2000`) or mock error responses (`?__status=500`) via query parameters in the dev server.
*   **Why it's Out of Scope:** This is a *pseudo-REST* utility focused on publishing. Adding rich developer-mock features introduces unnecessary logic and bloats the built-in development server. Developers needing mock status codes or artificial latency should use standard, dedicated mock proxies (like MSW - Mock Service Worker) or adopt a full-featured mock REST server, keeping this tool's code footprint minimal and hyper-focused.
