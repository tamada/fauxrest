# prest Conventions and Physical Mapping Specifications

`prest` is a **convention-over-configuration** static API generator that allows for the rapid construction of robust static RESTful APIs by following predetermined, intuitive conventions without the need for detailed configuration files.

This document defines the core philosophy, global configuration, basic conventions, and the physical file mapping rules.

---

## 1. Design Philosophy

- **Convention over Configuration**: Influenced by Ruby on Rails, `prest` allows developers to build APIs simply by organizing data structures in the `data/` directory, without explicit route definitions.
- **Data-Source Driven**: API structure and endpoints are defined automatically by the structure of the JSON data itself, not by static configuration files. Configuration files exist as an *option* for handling exceptions or customizations.

---

## 2. Global Configuration ($config.format)

The global output format is controlled via the `$config` key in the project root configuration file (e.g., `prest.json`).

`prest` supports **three consistent Output Formatting Modes** based on the delivery characteristics of static file servers. Using this `format` parameter instead of fragmented boolean flags ensures a unified and consistent physical file structure across the entire API.

```json
{
  "$config": {
    "format": "index"
  }
}
```

### Three Primary Output Formatting Modes (Delivery Formats)

1. **`index` Format (Default)**
   - **Content**: Both collections and individual resources are directory-indexed, outputting `index.json` files within directories.
   - **Benefit**: Provides perfectly clean URLs (e.g., `/endpoint`, `/endpoint/1`), working seamlessly on almost all static hosting platforms (S3, GitHub Pages, Netlify, Cloudflare Pages, etc.).

2. **`file` Format**
   - **Content**: All endpoints are output as extensionless plain files.
   - **Benefit**: Provides clean URLs. Best for modern CDNs (Netlify, Cloudflare Pages, etc.) that can automatically serve extensionless files with the correct `Content-Type: application/json` header.
   - **Collision Avoidance**: To avoid physical filesystem collisions (where a folder and a file cannot share the same name), this mode includes **Smart Fallback logic** that automatically replaces colliding collection files with `index.json` structures (see Chapter 5).

3. **`extension` Format**
   - **Content**: All endpoints are output with an explicit `.json` extension.
   - **Benefit**: While `.json` is exposed in the URL, this format is 100% compatible with all web servers and local filesystems without any redirection or configuration.

---

## 3. Basic Conventions

The core rules for automatically inferring and building API endpoints from data structures.

### Convention 1: Data sources become `Collection` APIs
Each JSON file placed in the `data/` directory maps directly to an API endpoint returning a collection of resources.
- **Condition**: `data/papers.json` exists.
- **Output**: `GET /api/papers` endpoint.

### Convention 2: `id` fields become `Individual Resource` APIs
If a collection contains objects with an `"id"` key, individual resource endpoints are automatically generated.
- **Condition**: Objects in `data/papers.json` have an `"id"` field.
- **Output**: `GET /api/papers/{id}` endpoint.

### Convention 3: The API root is the "Discovery Document"
The top-level hierarchy of the API automatically returns a discovery document listing all available endpoints.
- **Default**: `GET /api` returns a document describing the entire API structure.

### Convention 4: Ignored Files
Files and directories whose names begin with an underscore (`_`) or a dot (`.`) are explicitly ignored during the build process.
- **Benefit**: This allows you to safely store configuration files (e.g., `_config.json`), internal components, or drafts directly within the `data/` directory without exposing them as API endpoints.

---

## 4. Exception Handling via Configuration

For special cases that cannot be inferred by conventions, use the `$index` meta-key in the configuration file:

- **`"$index": "discovery"`**: Explicitly defines the `index.json` at this level as a "Discovery Document".
- **`"$index": "collection"`**: Explicitly defines the `index.json` as a standard collection endpoint.
- **`"$index": { "aggregate": ["/path1", ...] }`**: Merges and aggregates data from multiple collection sources into a single `index.json`.
- **`"$index": false`**: Explicitly disables the automatic generation of `index.json` at this level.

---

## 5. Physical File Mapping Rules (Smart Fallback Specification)

Operating systems have a physical rule: **"A file and a directory cannot share the same name at the same path."**
For example, in `file` mode, outputting a collection file `/api/papers` prevents the creation of a directory `/api/papers/` required to house individual resources like `/api/papers/{id}`.

To resolve this, `prest` applies **"Smart Fallback."**

### Smart Fallback Rules
- When compiling in `file` mode, **collections that contain sub-paths (like individual resource items) automatically fallback to `.../index.json` format** to avoid physical collisions.
- Single resources without sub-paths (e.g., `/api/profile`) are output as extensionless files (`api/profile`).
- This avoids collisions while maintaining clean URLs, as CDNs automatically resolve requests for `/api/papers` to the generated `/api/papers/index.json`.

### Physical File Mapping Table

| API Endpoint | `index` Format (Default) | `file` Format (Post-Smart Fallback) | `extension` Format |
| :--- | :--- | :--- | :--- |
| **`GET /api`** | `api/index.json` | `api/index.json` (fallback) | `api.json` |
| **`GET /api/papers`** | `api/papers/index.json` | `api/papers/index.json` (fallback) | `api/papers.json` |
| **`GET /api/papers/{id}`**| `api/papers/{id}/index.json` | `api/papers/{id}` | `api/papers/{id}.json` |
| **`GET /api/profile`** | `api/profile/index.json` | `api/profile` | `api/profile.json` |
