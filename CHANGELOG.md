# Changelog

All notable changes to this project are documented in this file.

The format is based on Keep a Changelog, and this project uses semantic
versioning while the public API is still stabilizing.

## [Unreleased]

### Breaking changes

- Renamed `PipelineResult::next` and `TransactionResult::next` to
  `next_result` to distinguish typed result decoding from iterator APIs.

### Improved

- Reduced protocol parser and integer-header allocation overhead for RESP2 and
  RESP3, and added an ignored allocation-report harness for repeatable local
  measurements.

### Changed

- Consolidated repository documentation, package metadata, examples, and
  benchmarks ahead of the 0.3 beta release.

## [0.3.0-alpha.1] - 2026-08-26

### Breaking changes

- Removed the unfinished optimized pool, command-builder, and RESP2 codec
  modules. The crate now has one supported transport and pooling path.
- RESP3 maps, sets, attributes, and blob values now preserve wire-format data
  without lossy string-only keys or UTF-8 coercion.

### Fixed

- Hardened connection lifecycle, pooling, stateful command isolation, and
  configuration handling ahead of the 0.3 release.

## [0.2.6] - 2026-05-14

### Fixed

- Replaced relative repository links in the crates.io README with absolute
  GitHub URLs so documentation links resolve from the published crate page.

## [0.2.5] - 2026-05-14

### Changed

- Bumped the crate version after the `0.2.4` publish.
- Updated installation documentation to use the `0.2` release line instead of
  hardcoding a patch version.
- Reworded the roadmap to point readers to `CHANGELOG.md` and crates.io for the
  latest published version.

## [0.2.4] - 2026-05-14

### Fixed

- Kept the codebase compatible with Rust and Clippy 1.95.
- Updated GitHub Actions to use Node 24-compatible actions and removed the
  Node.js 20 deprecation warning path.
- Replaced the Rust cache action that emitted the `punycode` deprecation warning
  with the official GitHub cache action.

### Changed

- Reworked public documentation into a smaller professional docs set.
- Added a root changelog and included it in the published crate package.

### Verified

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-targets`

## [0.2.3] - 2026-05-14

### Fixed

- Encode Redis command arguments as bulk strings while preserving standalone
  RESP value encoding.
- Preserve Redis TTL sentinel values through `ttl`.
- Fall back from `EVALSHA` to `EVAL` for `Script::execute` when Redis returns
  `NOSCRIPT`.
- Return Redis server errors inside pipeline result vectors instead of aborting
  the entire pipeline.
- Avoid sending `DISCARD` before `MULTI` for locally queued transactions.

### Changed

- Removed unused dev dependencies.
- Cleaned cargo-deny license and advisory configuration.

### Verified

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --all-targets`
- `cargo doc --workspace --no-deps --document-private-items`
- `cargo audit`
- `cargo deny check`
