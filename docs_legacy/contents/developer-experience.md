# Developer Experience (DX) Features

`prest` prioritizes local developer speed, seamless integration, and frontend safety through several advanced built-in features.

---

## ⚡ Internal Development Server (`serve`)

`prest` features a lightweight, high-performance internal development server that eliminates the need for external static servers (like Nginx, Live Server, or serve).

Start the server using:
```bash
prest serve --port 8080 --host 127.0.0.1
```

### Key DX Capabilities
*   **Automatic Content-Type Headers:** Ensures files are served with the correct `Content-Type: application/json` headers, preventing client fetch errors.
*   **Out-of-the-Box CORS:** Automatically attaches cross-origin headers (`Access-Control-Allow-Origin: *`) so you can query your local API from separate front-end dev domains (e.g. `localhost:3000`).
*   **Live Hot-Reloading:** Watches your raw input directory and config files for changes, triggers an immediate background rebuild, and re-serves the updated JSON payloads on-the-fly.

---

## 📘 TypeScript Type Generation (`.d.ts`)

To guarantee absolute type safety in your front-end applications, the `prest` compiler parses your raw JSON data structures during build time and automatically exports highly accurate TypeScript type definition files (`.d.ts`).

### Output Location
Type definitions are written automatically to:
```text
dist/types/
├── profile.d.ts
└── users.d.ts
```

### Usage Example
Import the generated types directly in your TypeScript client (e.g., React, Vue, or Next.js):

```typescript
import { User } from './dist/types/users';

async function fetchUser(id: number): Promise<User> {
  const response = await fetch(`https://api.domain.com/users/${id}`);
  return response.json();
}
```
This gives front-end teams immediate autocomplete and build-time validation out-of-the-box.

---

## 🏎️ Caching & Incremental Builds

Compiling huge JSON structures can bottleneck your local dev environment or slow down CI/CD pipelines. To prevent this, `prest` incorporates an incremental caching mechanism.

*   During a build, `prest` computes SHA-256 hashes of all input files and stores them in `.static-api-cache.json`.
*   During subsequent builds, `prest` checks these cached hashes.
*   Only files that have actually changed are re-compiled.
*   This slashes build times from seconds down to milliseconds, allowing local live-reload to remain virtually instantaneous.

---

## 🖼️ Media Asset Bundling & Path Rewriting

Raw JSON databases often contain relative references to local media files (e.g. `"avatar": "./images/bob-avatar.png"`).

When `prest` compiles your endpoints:
1.  It automatically scans compiled JSON values for relative file paths pointing to existing local media/assets.
2.  It copies these media files into the distribution folder: `/dist/api/assets/`.
3.  It rewrites the path inside the generated output JSON to the final relative URL: `"avatar": "/api/assets/bob-avatar.png"`.

This prevents broken image links on the client and packages your API data and associated media into a single self-contained unit ready for CDN distribution.
