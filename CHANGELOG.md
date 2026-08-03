# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions below 0.1.0 carry no compatibility guarantees: under Cargo's rules
every `0.0.x` release is incompatible with every other, so breaking changes can
and do appear in patch releases. They are called out under **Changed** so you
can see them before upgrading.

## [Unreleased]

### Changed

- **Breaking:** a `$filter` comparison whose two sides hold JSON kinds it
  cannot evaluate now fails the run instead of printing a warning to stderr and
  treating the condition as unmatched. This covers one field holding different
  kinds across records — the case where no single `$derive.type` keeps every
  record, so roughly half were dropped and the endpoint still published — and
  operators applied to a kind they cannot order, such as `gt` on two booleans.
  A no-op `"pattern": ".*"` over a numeric field is reported for the same
  reason, rather than yielding an empty collection per derived value.
  `null` and absent fields are exempt: a field left unset in some records is
  ordinary data, and `"value": null` is how a condition asks whether a field is
  unset. ([#3])

## [0.0.4] - 2026-08-01

### Changed

- **Breaking:** `$derive.type` now accepts only `string` and `int`. The
  `float`, `bool` and `auto` values that 0.0.3 shipped are rejected when the
  configuration loads. A derived value becomes a path segment, and those are
  the two kinds that makes sense for; a survey of the datasets then in
  `testdata/` found nine `int` fields usable as grouping keys, three `bool`
  fields (only ever `$filter` predicates, never path segments) and no `float`
  fields at all. `auto` also had a sharp edge: a value derived from a *string*
  field and inferred as a number stopped matching that field under `eq`,
  silently emptying the endpoint. `type` is an additive enum, so the omitted
  kinds can return later without invalidating a configuration that works today.
  ([#12])

  If your configuration uses one of the three, remove the `type` key or replace
  it with `string` or `int`.

### Fixed

- An invalid `$filter` regex no longer publishes the records it was meant to
  withhold. `regeq`/`regneq` patterns that could not compile were neither
  rejected at load time nor reported at runtime — the condition was treated as
  matched, so **every** record was emitted with exit code 0 and nothing on
  stderr. Patterns are now validated when the configuration loads, and a
  condition that cannot be evaluated aborts the build instead of failing open.
  ([#1], [#11])
- Ordering operators work on strings. `gt`, `gte`, `lt` and `lte` reduced both
  operands to `0.0`, so `gt`/`lt` matched nothing while `gte`/`lte` matched
  *everything* — a date-range endpoint over ISO-8601 strings silently published
  its whole collection. Numbers now compare numerically and strings
  lexicographically, which is the correct order for zero-padded, fixed-width
  formats such as `"2026-10-16"`. It remains meaningless for free-form text like
  `"April 2023"`. ([#2], [#13])
- Configuration errors name what is wrong. Every mistake inside a `$derive`,
  `$aggregate` or `$static` directive produced the same sentence — `data did
  not match any variant of untagged enum ...` — naming none of them. A rejected
  `type`, a misspelled key and a value of the wrong kind are now reported
  individually, for example ``unknown variant `auto`, expected `string` or
  `int` ``. ([#16], [#17], [#20])

### Performance

- `$filter` and `$derive` patterns are compiled once per pattern instead of once
  per record. Compiling costs far more than matching, and the waste multiplied
  across serializers. Measured over 200000 records with four regex conditions:
  6.33 s before, 0.47 s after. ([#4], [#11])

### Added

- A "Powered by FauxREST" badge, with copy-pasteable Markdown and HTML in the
  README. ([#18])
- `benches/filter_regex.rs`, a criterion benchmark over the `$filter` evaluation
  loop, and `testdata/bench`, the end-to-end equivalent. ([#11])
- `CONTRIBUTING.md`, plus `just` recipes for pre-push checks and formatting.
  ([#15])

### Internal

- The published crate no longer carries `testdata/`, `docs/`, `.github/` or
  `.vscode/`, and `Cargo.lock` is regenerated as part of the version bump so it
  cannot drift from `Cargo.toml`. Both were blocking `cargo publish`. ([#10])
- The repository is `cargo fmt` clean and CI enforces it. CI also runs on
  `pull_request`, so contributions from forks are checked — previously the
  workflow only triggered on `push`, which a fork cannot fire. ([#6], [#15])
- Documentation said `$derive.type` arrived in 0.0.4 and that omitting it
  reproduced pre-0.0.4 behaviour. Both halves were wrong: 0.0.4 never existed as
  a release, and the field shipped in 0.0.3. ([#17])

## [0.0.3] - 2026-07-28

### Added

- `$derive.type` converts a derived value to another scalar kind before it is
  deduplicated, turned into a path segment and substituted into `$filter`
  conditions. A `$derive.pattern` always extracts a string, so without this a
  derived value could never match a numeric field. Accepted `string`, `int`,
  `float`, `bool` and `auto`; narrowed in 0.0.4.
- A publishing workflow, and installation documentation to match.

### Fixed

- Corrected the `pseudo-rest-api` keyword, previously misspelled.

## [0.0.2] - 2026-07-13

### Added

- `$static` copies non-JSON files (images, CSS, fonts) from the data directory
  into each serializer destination, with allow/deny globs.
- `overwrite` guards a non-empty destination directory instead of clobbering it.
- `--no-minify` disables minification from the command line.
- Shell completions for bash, elvish, fish, powershell and zsh.
- A documentation site, built and deployed from `main`.
- Doc comments and examples across the public API.

### Fixed

- Command-line options take precedence over the configuration file
  (**CLI > config > default**). They were previously ignored when a
  configuration file was loaded.

## [0.0.1] - 2026-07-02

Initial release. Compiles JSON datasets into static API endpoints, with the
`json`, `typescript` and `sqlite` serializers, the `index`, `file` and
`extension` layouts, and the `_config.json` overlay schema.

[Unreleased]: https://github.com/tamada/fauxrest/compare/v0.0.4...HEAD
[0.0.4]: https://github.com/tamada/fauxrest/compare/v0.0.3...v0.0.4
[0.0.3]: https://github.com/tamada/fauxrest/compare/v0.0.2...v0.0.3
[0.0.2]: https://github.com/tamada/fauxrest/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/tamada/fauxrest/releases/tag/v0.0.1

[#1]: https://github.com/tamada/fauxrest/issues/1
[#2]: https://github.com/tamada/fauxrest/issues/2
[#3]: https://github.com/tamada/fauxrest/issues/3
[#4]: https://github.com/tamada/fauxrest/issues/4
[#6]: https://github.com/tamada/fauxrest/issues/6
[#10]: https://github.com/tamada/fauxrest/pull/10
[#11]: https://github.com/tamada/fauxrest/pull/11
[#12]: https://github.com/tamada/fauxrest/pull/12
[#13]: https://github.com/tamada/fauxrest/pull/13
[#15]: https://github.com/tamada/fauxrest/pull/15
[#16]: https://github.com/tamada/fauxrest/issues/16
[#17]: https://github.com/tamada/fauxrest/pull/17
[#20]: https://github.com/tamada/fauxrest/pull/20
[#18]: https://github.com/tamada/fauxrest/pull/18
