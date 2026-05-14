# Contributing

Contributions should keep the crate reliable, documented, and easy to verify.

## Development Setup

Install Rust 1.82.0 or newer and run Redis locally for integration tests:

```bash
docker run --rm -p 6379:6379 redis:7
```

Build and test:

```bash
cargo check --workspace --all-targets
cargo test --workspace --all-targets
```

## Quality Gates

Run these before opening a pull request:

```bash
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo doc --workspace --no-deps --document-private-items
cargo audit
cargo deny check
```

## Code Guidelines

- Keep public APIs consistent with existing naming and return types.
- Prefer typed command builders over ad hoc command construction.
- Preserve Redis server semantics unless the public API documents otherwise.
- Add focused tests for behavior changes.
- Avoid broad refactors in feature or bugfix patches.

## Documentation Guidelines

- Keep docs professional and concise.
- Do not use emoji in headings, status labels, or prose.
- Verify examples against the current public API.
- Update `CHANGELOG.md` for user-visible changes.

## Pull Requests

Include:

- the problem being solved
- the behavior change
- tests or verification commands run
- any compatibility notes
