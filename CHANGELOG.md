# Changelog

All notable changes to this project are documented in this file.

The format is based on Keep a Changelog, and this project uses semantic
versioning while the public API is still stabilizing.

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
