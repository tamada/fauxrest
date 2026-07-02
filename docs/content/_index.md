---
title: "FauxREST"
date: "2026-06-30"
---

## About fauxrest

`fauxrest` is a convention-over-configuration static API generator written in Rust.
It compiles JSON datasets into production-ready static API endpoints for CDN hosting.

### Why fauxrest

1. Infinite scalability through static asset delivery on edge networks.
2. Low operating cost by removing application servers and databases for read APIs.
3. Low maintenance with no runtime patching, pooling, or server management.
4. Strong DX through fast builds, incremental compilation, and generated typings.

### Design Philosophy

- Convention over configuration: API routes are inferred from data structure by default.
- Data-source driven: data files define endpoint shape; config only adjusts exceptions.
- Static-first delivery: output is optimized for static hosts and object storage.

### Core Conventions

- `data/papers.json` produces a collection endpoint at `/api/papers`.
- Records with `id` also produce item endpoints such as `/api/papers/{id}`.
- API root (`/api`) acts as a discovery document for available endpoints.
- Files and directories prefixed with `_` or `.` are ignored by the builder.

### Delivery Model

`fauxrest` can emit multiple outputs in one build, such as JSON for API delivery,
TypeScript modules for frontend builds, and SQLite for offline query workloads.
