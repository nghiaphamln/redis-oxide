# Repository Guidelines

## Project Structure & Module Organization

This is a Rust workspace with one crate in `redis-oxide/`. Core library code is
under `redis-oxide/src/`, with Redis features split by module: `client.rs`,
`connection.rs`, `cluster.rs`, `pool.rs`, `pipeline.rs`, `transaction.rs`,
`streams.rs`, `script.rs`, and command groups in `src/commands/`. Protocol and
shared types live in `src/protocol/` and `src/core/`.

Tests are in `redis-oxide/tests/`, benchmarks in `redis-oxide/benches/`, and
examples in both root `examples/` and `redis-oxide/examples/`. User-facing
documentation is kept in `README.md`, `CHANGELOG.md`, and `docs/`.

## Build, Test, and Development Commands

Use the workspace root for all commands:

- `cargo fmt --all --check`: verify Rust formatting.
- `cargo check --workspace --all-targets`: compile all targets without building binaries.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: enforce lint cleanliness.
- `cargo test --workspace --all-targets`: run unit and integration test targets.
- `cargo doc --workspace --no-deps --document-private-items`: build local docs.
- `cargo audit` and `cargo deny check`: run security and policy checks.

In the local agent environment, prefix shell commands with `rtk`, for example
`rtk cargo test --workspace --all-targets`.

## Coding Style & Naming Conventions

Follow `rustfmt.toml` and keep Clippy warnings at zero. Use idiomatic Rust
naming: modules and functions in `snake_case`, types and traits in `PascalCase`,
and constants in `SCREAMING_SNAKE_CASE`. Keep public APIs documented; the crate
warns on missing docs and denies warnings in library builds.

## Testing Guidelines

Place integration tests in `redis-oxide/tests/` with descriptive names such as
`test_pipeline.rs` or `test_resp3_protocol.rs`. Prefer focused tests for command
behavior, protocol encoding, error handling, and Redis response parsing. Tests
that require a live Redis server should document the required `REDIS_URL` and
avoid making default CI jobs depend on external services.

## Commit & Pull Request Guidelines

Git history uses conventional-style prefixes such as `fix:`, `feat:`, `docs:`,
`refactor:`, `chore:`, and `bump:`. Keep commits scoped and describe the user
visible effect, for example `fix: preserve ttl sentinel values`.

Pull requests should include a concise summary, changed areas, linked issues if
available, and the verification commands run. Update `CHANGELOG.md` for release
or user-visible changes, and keep CI workflow changes minimal and justified.
