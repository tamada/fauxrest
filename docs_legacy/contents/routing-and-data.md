# Routing and Data Transformations

`prest` is designed to be highly flexible, offering three configuration tiers for data routing, and supporting multiple REST transformation patterns and sanitization parameters.

---

## 🛠️ Configuration Tiers

### Tier 1: Zero-Config (Convention over Configuration)
If run without any configuration file, `prest` automatically infers endpoints based on the file structures of your raw JSON data folder (defaulting to `./data`):

*   **Object Files (e.g., `profile.json`):** Compiles directly to `/profile/index.json` (maps to `/profile`).
*   **Array Files (e.g., `users.json`):** Compiles the full array to `/users/index.json` (maps to `/users`). If the elements in the array contain an `"id"` key, individual item files are automatically created at `/users/:id/index.json` (maps to `/users/:id`).

### Tier 2: Shorthand Routing (`routes.yml`)
For basic routing overrides, custom paths, or advanced transformations, users can write a clean, simple YAML configuration:

```yaml
"/users/:id": "data/users.json"
"/blog/posts": "data/db.json$.posts" # Extracts sub-nodes via JSONPath
```

### Tier 3: Programmatic Routing (`api.config.js`)
For highly complex, code-based data transformation, `prest` evaluates native JavaScript or TypeScript configuration files. This prevents users from needing to learn a proprietary DSL:

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

## 🧬 Core REST Transformation Patterns

When configuring custom routes, `prest` processes datasets using four core structural patterns:

### 1. Single Object (`type: single`)
Copies a single targeted JSON object directly to the route path.
*   *Input:* `{"name": "Alice"}`
*   *Output:* `/endpoint/index.json` containing the object.

### 2. Collection (`type: list`)
Compiles a JSON array directly as a list endpoint.
*   *Input:* `[{"name": "Bob"}, {"name": "Charlie"}]`
*   *Output:* `/endpoint/index.json` containing the full array.

### 3. Item Detail (`type: detail`)
Evaluates an input array, loops over each element, and exports a dedicated static file for every item by substituting a dynamic segment (e.g., `:id`) with a specified key.
*   *Path:* `/users/:id`
*   *Output files:* `/users/1/index.json`, `/users/2/index.json` etc.

### 4. Grouping (`type: group`)
Groups array objects by a specified field value and outputs separate sub-category lists.
*   *Example:* Grouping users by role `/users/role/:role`
*   *Output files:* `/users/role/admin/index.json` (contains only admins), `/users/role/editor/index.json` (contains only editors).

---

## 🔍 Advanced Node Targeting (JSONPath)

Sometimes datasets are nested inside single monolithic database files (like a mock lowdb file or local CMS output). `prest` allows the `source` parameter to target nested data structures directly using standard JSONPath queries:

```yaml
# Target the nested activities array within db.json
"/activities": "data/db.json$.activities"
```

---

## 🧼 Data Sanitization (`pick` & `omit`)

To protect sensitive values (such as passwords, secret tokens, or drafts) and minimize the compiled file size, `prest` supports two sanitization parameters:

*   **`pick` (Allowlist):** Keep only the specified keys. All other keys are stripped.
*   **`omit` (Denylist):** Explicitly strip specified keys. All other keys are preserved.

### YAML Example
```yaml
"/users/:id":
  source: "data/users.json"
  omit:
    - "password_hash"
    - "raw_token"
```
This ensures sensitive raw data never leaks into your public CDN.
