# fauxrest User Guide and Deployment Procedures

`fauxrest` is a zero-maintenance, blazingly fast Static API Generator written in Rust. It compiles raw JSON datasets into structured, production-ready static API endpoints deployable directly to cost-effective, infinitely scalable CDNs (such as GitHub Pages, Cloudflare Pages, Netlify, AWS S3, etc.).

---

## 1. Key Value Propositions

1. **Infinite Scalability**: Static JSON files are distributed natively over globally replicated edge networks, eliminating database scaling bottlenecks.
2. **Ultra-low Cost**: Hosting static files on modern CDNs is completely free for massive traffic volumes.
3. **No Server Maintenance**: Eliminates the need for database connection pooling, SSL renewals, or server patches.
4. **Superior DX**: Features incremental builds, automated TypeScript type generation, and a live-reloading internal development server.

---

## 2. Target Audience

- **Frontend Developers**: Consume mock or production-ready, read-only API endpoints with immediate client-side TypeScript safety and no backend setup required.
- **Data & Backend Engineers**: Structure simple raw data sources, declare custom routing using YAML/JS, filter via JSONPath, and sanitize public endpoints.
- **DevOps & SREs**: Utilize optimized build pipelines, incremental caching for fast CI/CD, and deploy to robust, global CDNs.

---

## 3. Installation

To build `fauxrest` from source, ensure Rust is installed.

```bash
# Clone the repository
git clone https://github.com/username/fauxrest.git
cd fauxrest

# Build in release mode
cargo build --release
```

The compiled binary will be located at `target/release/fauxrest`.

---

## 4. Quick Start

### Step 1: Prepare Data
Create a `data/` directory and add JSON datasets:

```json
// data/profile.json (Single object)
{
  "name": "Alice",
  "role": "Developer"
}
```

### Step 2: Build
Run the build command:
```bash
fauxrest build ./data fauxrest.json
```

### Step 3: Verify Output
`fauxrest` generates structured files in the designated directory based on the serializers defined in `fauxrest.json`.

---

## 5. Development Server (`serve`)

`fauxrest` includes a lightweight, internal development server for local testing.

```bash
fauxrest serve ./data --port 8080
```

### DX Capabilities
- **Auto Content-Type**: Sets correct headers (e.g., `application/json`).
- **Local CORS**: Automatically attaches CORS headers for local frontend development.
- **Live Reloading**: Watches `data/` and config files, triggering immediate incremental rebuilds on changes.

---

## 6. Deployment Procedures

### GitHub Pages
1. Build your endpoints: `fauxrest build ./data fauxrest.json`
2. Commit/push the output directory (e.g., `./dist`) to your `gh-pages` branch.
3. Configure GitHub Pages to serve from that branch.

### Cloudflare Pages / Netlify
1. Connect your repository to the service.
2. Build Command: `fauxrest build ./data fauxrest.json`
3. Publish Directory: (match your config)
4. CORS headers are applied automatically via generated `_headers` files.

### AWS S3 / CloudFront
1. Build: `fauxrest build ./data fauxrest.json`
2. Upload to S3: `aws s3 sync ./dist s3://my-api-bucket/`
3. Enable "Static Website Hosting" on S3.
4. Use CloudFront distribution for HTTPS and CDN caching.
