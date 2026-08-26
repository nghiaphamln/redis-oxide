# Contributing

Contributions should preserve documented behavior, keep public APIs coherent,
and include focused verification.

## Development setup

Install Rust 1.82 or newer and run Redis locally:

```bash
docker run --rm -p 6379:6379 redis:7
```

## Required checks

Run these from the workspace root before opening a pull request:

```bash
cargo fmt --all --check
cargo check --locked --workspace --all-targets --all-features
cargo clippy --locked --workspace --all-targets --all-features -- -D clippy::all -D clippy::pedantic -D clippy::nursery -D warnings
cargo test --locked --workspace --all-targets --all-features
cargo test --locked --doc --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps --document-private-items
cargo deny --locked check
cargo package --locked -p redis-oxide --allow-dirty
```

## Guidelines

- Keep feature and fix patches scoped to the reported behavior.
- Preserve Redis server semantics unless a public API documents a deliberate
  conversion.
- Add focused tests for changed command, protocol, timeout, or routing behavior.
- Keep prose concise, use no emoji in source code, and link to canonical
  examples instead of duplicating long tutorials.
- Update the changelog when a user-visible change is planned for release.

## Pull requests

Include the problem, behavior change, compatibility impact, and verification
commands. Release commits also require package verification and a version tag.
