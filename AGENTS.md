# Repository Guidelines

## Project Structure & Module Organization

This is a Rust workspace with one crate in `redis-oxide/`. Core library code is
under `redis-oxide/src/`, with Redis features split by module: `client.rs`,
`connection.rs`, `cluster.rs`, `pool.rs`, `pipeline.rs`, `transaction.rs`,
`streams.rs`, `script.rs`, `sentinel.rs`, and command groups in
`src/commands/`. Protocol and shared types live in `src/protocol/` and
`src/core/`.

Tests are in `redis-oxide/tests/`. The retained Criterion benchmark is
`redis-oxide/benches/protocol_bench.rs`; runnable examples are in
`redis-oxide/examples/`. User-facing documentation is kept in `README.md`,
`CHANGELOG.md`, `CONTRIBUTING.md`, and `docs/`. `Cargo.lock` is tracked to keep
CI, MSRV, and package verification reproducible.

## Build, Test, and Development Commands

Run commands from the workspace root. In the local agent environment, prefix
Cargo commands with `rtk`.

- `cargo fmt --all --check`: verify Rust formatting.
- `cargo check --locked --workspace --all-targets --all-features`: compile all targets.
- `cargo clippy --locked --workspace --all-targets --all-features -- -D clippy::all -D clippy::pedantic -D clippy::nursery -D warnings`: enforce strict lint cleanliness.
- `cargo test --locked --workspace --all-targets --all-features`: run unit and standalone integration tests.
- `cargo test --locked --doc --workspace --all-features`: run documentation tests.
- `RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps --document-private-items --all-features`: build strict Rust documentation.
- `cargo deny --locked check`: run required advisory, license, ban, and source policy checks.
- `cargo package --locked -p redis-oxide --allow-dirty`: verify the publishable package locally.
- `cargo bench --locked -p redis-oxide --bench protocol_bench --no-run`: verify the supported benchmark target builds.

Run `cargo audit` when it is installed as an additional local security check; it
is not a required CI gate.

## Coding Style, Documentation, and Repository Hygiene

Follow `rustfmt.toml` and keep Clippy clean across `all`, `pedantic`, and
`nursery`. Do not add `allow(clippy::...)` suppressions: fix, redesign, or
split the code that triggers the lint. Use idiomatic Rust naming: modules and
functions in `snake_case`, types and traits in `PascalCase`, and constants in
`SCREAMING_SNAKE_CASE`.

Keep public APIs documented. The library denies warnings, so missing public
documentation and broken rustdoc links fail the build. Do not use emoji in Rust
source, including doc comments, examples, benchmarks, and tests.

Keep `Cargo.lock` updated and use `--locked` for verification. Do not commit
generated artifacts such as `target/` output or `redis-oxide/clippy_*.txt`.
The project is MIT-only; preserve the packaged `redis-oxide/LICENSE-MIT` file.

## Testing Guidelines

Place focused integration tests in `redis-oxide/tests/`, with descriptive names
such as `test_pipeline.rs` or `test_resp3_protocol.rs`. Cover command behavior,
protocol encoding and parsing, error handling, timeout behavior, and Cluster
routing changes.

Standalone integration tests use `REDIS_URL` and default to
`redis://localhost:6379`. CI intentionally provides a Redis service for these
tests. Cluster and Sentinel end-to-end tests in `test_topology_e2e.rs` are
ignored locally and run in CI through
`redis-oxide/tests/fixtures/redis-topologies.compose.yml`; document any required
topology environment variables with the test.

## Commit and Pull Request Guidelines

Use conventional-style prefixes such as `fix:`, `feat:`, `docs:`, `refactor:`,
`chore:`, and `bump:`. Keep commits scoped and describe the user-visible effect.

Pull requests must include a concise summary, changed areas, compatibility
impact, linked issue or milestone when applicable, and verification commands.
Update `CHANGELOG.md` for release or user-visible changes. Keep CI changes
minimal and justified.

When a change touches a protocol parser, serialization path, pool, or other hot
path, include a before/after Criterion comparison against `main` in the PR
description. State the benchmark environment, confidence interval, and whether
the result is a deliberate optimization or an incidental cleanup benefit.
