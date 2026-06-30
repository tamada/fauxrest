# fauxrest

[![Version](https://img.shields.io/badge/Version-0.0.1-blue)](https://github.com/tamada/fauxrest/releases/tag/v0.0.1)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue?logo=spdx)](LICENSE)

[![Coverage Status](https://coveralls.io/repos/github/tamada/fauxrest/badge.svg?branch=releases/v0.0.1)](https://coveralls.io/github/tamada/fauxrest?branch=main)
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-c45508?logo=rust)](https://www.rust-lang.org/)
[![Docker](https://img.shields.io/badge/Container-quay.io/tama5/fauxrest:latest-2496ED?logo=docker)](https://quay.io/repository/tama5/fauxrest)

![logo](.github/assets/logo.svg)

> **Pseudo-REST Static API Generator** — Compile raw JSON datasets into structured, production-ready static API endpoints deployable directly to cost-effective, infinitely scalable CDNs (GitHub Pages, Cloudflare Pages, Netlify, AWS S3, etc.).

`fauxrest` is a zero-maintenance, blazingly fast command-line utility written in Rust. It eliminates the need for dynamic server processes (such as Node.js or Python) and databases for read-only APIs, enabling infinite scalability and sub-millisecond response times at zero hosting cost.

---

## 🚀 Features

* **Two Configuration Tiers:** From zero-config auto-inference (Tier 1) to shorthand JSON routes (Tier 2).
* **Flexible Routing Patterns:** Support Single Object, Collections (Lists), Item Details (via URL parameters), and Grouping transformations out of the box.
* **Advanced Data Cleaning:** Deep JSONPath extraction, along with robust sanitization using `pick` (allowlist) and `omit` (denylist).
* **Multi-Platform Target Formats:** Output formats tailored for all static hosting environments, supporting trailing slashes, directory index resolutions, extensionless files, or fully bundled files.
* **Asset Bundling & Path Rewriting:** Scans compilation output for local asset references (e.g., images), copies them to the distribution folder, and rewrites path links safely.

---

## ⚓️ Installation

### 🍺 Homebrew

To install `fauxrest` into your local environment, type the following command.

```sh
# install fauxrest
brew install tamada/tap/faurest
```

### 🐳 Container image

[![Docker](https://img.shields.io/badge/Container-quay.io/tama5/fauxrest:latest-2496ED?logo=docker)](https://quay.io/repository/tama5/fauxrest)

Fauxrest also supports container images distributed on [Quay.io](https://quay.io/repository/tama5/fauxrest). Use it with the following command (in the following example, we use [`docker`](https://www.docker.com); however, [`podman`](https://podman.io), [`finch,`](https://runfinch.com) and/or [`apple/container`](https://github.com/apple/container) might be supported).

```sh
docker run -it --rm -v $PWD:/opt quay.io/tama5/faurest:latest 
```

Options and arguments should follow the above command.

### 💪 Compile yourself

To compile `fauxrest` from source, ensure you have Rust installed.

```bash
# Clone the repository
git clone https://github.com/username/fauxrest.git
cd fauxrest

# Build in release mode
cargo build --release
```

The compiled binary will be located in `target/release/fauxrest`.

---

## 🚦 Quick Start (Zero-Config Mode)

No configuration files are needed. Organize your JSON data into a directory:

```text
data/
├── profile.json       # direct single object
└── users.json         # list of user objects with "id"
```

Then run `fauxrest`:

```bash
fauxrest data -d dist
```

`fauxrest` will scan `./data`, infer the routes, and output:

```text
dist/
├── profile/
│   └── index.json     # Resolves to /profile
└── users/
    ├── index.json     # Resolves to /users
    ├── 1/
    │   └── index.json # Resolves to /users/1 (derived from ID)
    └── 2/
        └── index.json # Resolves to /users/2 (derived from ID)
```

---

## 🛠️ Configuration Tiers

### Tier 1: Zero-Config (Convention over Configuration)
*   **Object files (`profile.json`):** Mapped to `/profile`.
*   **Array files (`users.json`)** are mapped to `/users/index.json` (list view), and if elements contain an `id` field, individual items map to `/users/:id` (detail view).

### Tier 2: Shorthand Routing (`_config.json`)
For basic routing overrides, create a simple `_config.json` configuration in the `data` directory. The files starting with a dot (`.`) or an underscore (`_`) are ignored for the resultant JSON files.

---

### ⚡ Output Formats (`--format`)

To ensure seamless routing behavior across different CDN and static hosts:

*   `--format index` (Default): Outputs `/endpoint/index.json`. Perfect for standard servers resolving `/endpoint/`.
*   `--format file`: Outputs `/endpoint` file with no file extension.
*   `--format extension`: Outputs `/endpoint.json`. Zero-risk option compatible with every hosting platform.

---

## 🛑 Out of Scope (Non-Functional Scope)

To maintain simplicity, high speed, and ease of deployment, certain features are explicitly **out of scope**:

1.  **Dynamic Write Operations (`POST`, `PUT`, `DELETE`):** `fauxrest` is purely a static read-only generator. If your clients require dynamic, stateful database updates, you should migrate to a standard, dynamic API backend.
2.  **API Versioning (Simultaneous `/v1` and `/v2` branches):** Maintaining legacy endpoint schemas within a static compiler adds unnecessary overhead. If multiple active versions must be supported, standard dynamic API gateways or hosting-level rewrite layers should be utilized.
3.  **Advanced Mock Server Latency or Status Emulation:** This is a deployment publisher, not a test suite mock framework. If you need rich latency simulation (`?_delay=2000`) or status-code mocking, please use standard client-side libraries like MSW (Mock Service Worker).

---

## 📖 Specifications

For comprehensive technical architecture design, command line args, and subcommands, see:
*   [Full Architectural Specification](./.github/assets/spec.md)
*   [Command Line Interface (CLI) Specification](./.github/assets/cli_spec.md)

---

## 😀 About

### 👩‍💼 Developers 👨‍💼

- Haruaki Tamada ([@tamada](https://github.com/tamada))
- Google Gemini, GitHub Copilot

### 📄 License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.

### 📛 The project name (`fauxrest`)

The project name `fauxrest` is pronounced /fɔːrɪst/ or /foʊrɪst/, which is the same as "forest".
And it is derived from Faux (meaning `fake' in French) and REST.

### 🎃 Icon

![logo](.github/assets/logo.svg)

This logo is based on the one published on [SVGRepo](https://www.svgrepo.com/svg/476993/forest).
