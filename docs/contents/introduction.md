# Introduction to `prest`

`prest` is a zero-maintenance, blazingly fast Static API Generator written in Rust. It compiles raw JSON datasets into structured, production-ready static API endpoints deployable directly to cost-effective, infinitely scalable CDNs (such as GitHub Pages, Cloudflare Pages, Netlify, AWS S3, etc.).

By serving your read-only APIs as pre-rendered static JSON files, you eliminate database scaling problems, remove server-side security vulnerabilities, and achieve sub-millisecond response times at practically zero hosting cost.

---

## 🎯 Target Audience Map

`prest` is built to optimize workflows across several development roles:

*   **Frontend Developers:** Want to consume mock or production-ready, read-only API endpoints with immediate client-side TypeScript safety and no backend setup.
*   **Data & Backend Engineers:** Want to structure simple raw data sources, declare custom routing rules using YAML or JavaScript, filter deep JSON structures via JSONPath, and sanitize public endpoints.
*   **DevOps & SREs:** Want to set up highly optimized build pipelines, utilize incremental caching to speed up CI/CD, and deploy static assets to robust global CDNs.
*   **Contributors:** Want to extend the `prest` compiler and local development server, adhering to high-quality standards.

---

## 🚀 Key Value Propositions

1.  **Infinite Scalability:** Static JSON files are distributed natively over globally replicated edge networks.
2.  **Ultra-low Cost:** Hosting static files on modern CDNs is completely free for massive traffic volumes.
3.  **No Server Maintenance:** Say goodbye to database connection pooling, query optimization, SSL renewals, or server patches.
4.  **Instant Developer Feeback:** Features a lightweight development server (`serve` subcommand) with live reloading on file changes.

---

## 📦 Installation

To build `prest` from source, ensure you have Rust installed on your system.

```bash
# Clone the repository
git clone https://github.com/username/prest.git
cd prest

# Build in release mode
cargo build --release
```

The compiled binary will be placed at `target/release/prest`.

---

## 🚦 Quick Start

Create a folder named `data/` and add some raw JSON datasets:

```json
// data/profile.json
{
  "name": "Alice",
  "role": "Developer"
}
```

```json
// data/users.json
[
  { "id": 1, "name": "Bob" },
  { "id": 2, "name": "Charlie" }
]
```

Run `prest` to compile your first static API:

```bash
prest
```

`prest` will automatically infer the schema structures and output:

```text
dist/
├── profile/
│   └── index.json     # Resolves to /profile
└── users/
    ├── index.json     # Resolves to /users
    ├── 1/
    │   └── index.json # Resolves to /users/1
    └── 2/
        └── index.json # Resolves to /users/2
```
