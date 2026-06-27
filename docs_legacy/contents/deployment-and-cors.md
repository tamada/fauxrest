# Deployment and CORS Configuration

`prest` is purpose-built to output production-grade API files that can be deployed directly to static web hosts and globally distributed CDNs. This guide details target formats, CORS mechanisms, and specific deployment procedures.

---

## ⚡ Output Routing Formats (`--format`)

Different web servers and static CDNs resolve extensionless routes and directory indices differently. To maintain maximum compatibility across all environments, `prest` provides four compile-time formats:

| Format Option | Description | Target Host Compatibility |
| :--- | :--- | :--- |
| `--format index` *(Default)* | Outputs `/endpoint/index.json`. Static servers automatically resolve requests to `/endpoint` or `/endpoint/`. | **Excellent:** Works on almost all platforms (GitHub Pages, Netlify, Vercel, AWS S3, Cloudflare Pages). |
| `--format file` | Outputs `/endpoint` as an extensionless file. | **Good:** Perfect for modern hosts that handle extensionless MIME types natively (Netlify, Cloudflare Pages). |
| `--format extension` | Outputs `/endpoint.json`. Avoids all trailing-slash redirection logic. | **100% Compatible:** Works flawlessly on any file-server, but exposes `.json` in the URL. |
| `--format bundle` | Merges all routes and endpoints into a single `/db.json` file. | **Specialized:** Best for small client-side applications that load the entire database in one initial fetch. |

---

## 🔒 Out-of-the-Box CORS & Platform Headers

When serving your API from a static CDN domain (e.g., `api.example.com`) and querying it from your frontend domain (e.g., `example.com`), browsers enforce Cross-Origin Resource Sharing (CORS) rules.

To allow secure cross-origin requests, `prest` automatically generates platform-specific configuration headers during build-time:

### 1. Cloudflare / Netlify (`_headers` file)
Generates a `_headers` file in the build directory containing wildcards:
```text
/*
  Access-Control-Allow-Origin: *
  Access-Control-Allow-Methods: GET, OPTIONS
  Access-Control-Allow-Headers: Content-Type
```

### 2. Vercel (`vercel.json` file)
Generates a `vercel.json` configurations file automatically:
```json
{
  "headers": [
    {
      "source": "/(.*)",
      "headers": [
        { "key": "Access-Control-Allow-Origin", "value": "*" },
        { "key": "Access-Control-Allow-Methods", "value": "GET, OPTIONS" }
      ]
    }
  ]
}
```

This prevents your browsers from blocking API requests on your hosted domains with no server configuration required.

---

## 🌐 Deploying to Popular Hosting Environments

### GitHub Pages
1.  Compile your files with the default index format:
    ```bash
    prest --dest ./dist
    ```
2.  Commit the `./dist` folder (or push it to a dedicated `gh-pages` branch).
3.  In your GitHub repository settings, configure GitHub Pages to serve from that folder/branch.

### Cloudflare Pages / Netlify
1.  Connect your repository directly to Netlify or Cloudflare Pages.
2.  Set the **Build Command** to: `prest` (using the prebuilt binary or Cargo script).
3.  Set the **Publish Directory** to: `./dist`.
4.  CORS headers are applied automatically via the built-in `_headers` generation.

### AWS S3 / CloudFront
1.  Build your endpoints:
    ```bash
    prest --dest ./dist
    ```
2.  Upload the contents of `./dist` to your S3 bucket:
    ```bash
    aws s3 sync ./dist s3://my-api-bucket/ --acl public-read
    ```
3.  Enable "Static Website Hosting" on your S3 bucket.
4.  Optionally, configure a CloudFront distribution pointing to S3 to enable HTTPS and CDN caching.
