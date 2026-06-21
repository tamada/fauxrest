# Command Line Interface (CLI) Specification for `prest`

`prest` is a Static API Generator written in Rust. It compiles raw JSON datasets into optimized, static JSON API structures ready for CDN deployment, and includes a lightweight local development server.

## Synopsis

```bash
prest [COMMAND] [OPTIONS]
```

If no command is provided, `prest` defaults to the `build` subcommand.

---

## Global Options

*   `-h, --help`  
    Prints help information for `prest` or the specified subcommand.
*   `-V, --version`  
    Prints version information.

---

## Subcommands

### 1. `build`
Compiles source JSON data into structured static JSON endpoints ready for CDNs.

#### Usage
```bash
prest build [OPTIONS]
# OR (by default)
prest [OPTIONS]
```

#### Options
*   `-i, --inputs <PATH>`  
    Path to the input data directory or a single JSON file.  
    *Default:* `./data`

*   `-c, --config <PATH>`  
    Path to the configuration file (e.g., `routes.yml` or `api.config.js`).  
    *Default:* Searches for `api.config.js` or `routes.yml` in the current directory.

*   `-d, --dest <PATH>`  
    Path to the output/destination directory where static files will be generated.  
    *Default:* `./dist`

*   `-f, --format <FORMAT>`  
    Output format style for API routing compatibility on static hosts.  
    *Allowed values:*
    *   `index`: Outputs `/endpoint/index.json` (resolves `/endpoint` and `/endpoint/` on standard static hosts).
    *   `file`: Outputs `/endpoint` as an extensionless file.
    *   `extension`: Outputs `/endpoint.json` (avoids trailing slash / index resolution issues).
    *   `bundle`: Compiles all endpoints into a single `/db.json` file.  
    *Default:* `index`

*   `--no-cors`  
    Disables automatic CORS configuration and prevents writing platform headers (like `_headers` or `vercel.json`).

*   `--no-types`  
    Disables the automatic generation of TypeScript type definitions (`.d.ts`).

*   `--no-cache`  
    Disables incremental builds. Forces a complete rebuild of all endpoints from scratch, ignoring cached hashes.

---

### 2. `serve`
Starts a lightweight, internal development server serving the generated static API endpoints. It features hot-reloading (live-rebuilds on file changes) and serves appropriate JSON content types and CORS headers.

#### Usage
```bash
prest serve [OPTIONS]
```

#### Options
*   `-i, --inputs <PATH>`  
    Path to the input data directory or a single JSON file.  
    *Default:* `./data`

*   `-c, --config <PATH>`  
    Path to the configuration file.  
    *Default:* Searches for `api.config.js` or `routes.yml` in the current directory.

*   `-p, --port <PORT>`  
    The port on which the development server will listen.  
    *Default:* `8080`

*   `-H, --host <HOST>`  
    The IP address/host to bind the server to.  
    *Default:* `127.0.0.1`

*   `-f, --format <FORMAT>`  
    The routing format compatibility to emulate during serving.  
    *Allowed values:* `index`, `file`, `extension`, `bundle`  
    *Default:* `index`

---

## Examples

### Zero-Configuration Build (Tier 1)
Scan `./data` directory, infer RESTful routes automatically, and output static endpoints to `./dist` using default `/index.json` structure:
```bash
prest
```

### Build with Custom Data Source & Output Formats
Build endpoints using a custom data folder, outputting extensionless files into a `./build` directory:
```bash
prest build --inputs ./raw-json-data --dest ./build --format file
```

### Build using a Custom Route Configuration
Explicitly specify a YAML routing file:
```bash
prest build --config ./config/routes.yml
```

### Local Development Server
Launch the local dev server on port `3000` with live-rebuild enabled:
```bash
prest serve --port 3000
```

---

## Exit Codes

*   `0`: Success.
*   `1`: General/runtime error (e.g., config parsing failure, IO error).
*   `2`: Command line argument parsing error (Rust CLI standard).
