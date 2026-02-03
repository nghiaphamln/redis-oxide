# AGENTS.md - redis-oxide Development Guide

This file provides guidelines for AI agents working on the redis-oxide codebase.

## Build Commands

```bash
# Full workspace build
cargo build -p redis-oxide

# Build with all features
cargo build -p redis-oxide --all-features

# Build benchmarks (requires internal-optimizations feature)
cargo build --benches --features internal-optimizations
```

## Linting

```bash
# Run clippy on library (REQUIRED before commit)
cargo clippy -p redis-oxide

# Run clippy on ALL targets including benchmarks (REQUIRED)
cargo clippy --all-targets --all-features -- -D warnings

# Check formatting
cargo fmt --check

# Auto-format code (REQUIRED before commit)
cargo fmt
```

## Testing

```bash
# Run all library tests
cargo test -p redis-oxide --lib

# Run all tests (lib + integration)
cargo test -p redis-oxide

# Run a single test
cargo test -p redis-oxide --lib test_name
cargo test -p redis-oxide --test integration_tests test_name

# Run tests with output
cargo test -p redis-oxide --lib -- --nocapture

# Run benchmarks
cargo bench -p redis-oxide
cargo bench -p redis-oxide --benches protocol_bench

# Run integration tests (requires Redis)
REDIS_URL=redis://localhost:6379 cargo test -p redis-oxide --test integration_tests
```

## Code Style Guidelines

### Imports
- Use absolute paths with `crate::` for internal imports
- Group imports: std -> external -> internal
- Avoid re-exports unless for public API stability
- Do NOT use full paths like `redis_oxide::core::value::RespValue` - use `crate::RespValue`

### Formatting
- Run `cargo fmt` before committing (MANDATORY)
- Max line length: 100 characters (default rustfmt)
- Use 4 spaces for indentation
- Use `#[allow(...)]` sparingly - only when intentional

### Naming Conventions
- **Structs/Enums**: UpperCamelCase (e.g., `Client`, `PoolStats`)
- **Functions/Methods**: snake_case (e.g., `execute_command`)
- **Constants**: SCREAMING_SNAKE_CASE (e.g., `MAX_REDIRECTS`)
- **Type Parameters**: UpperCamelCase (e.g., `T: Command`)
- **Modules**: snake_case (e.g., `pool`, `protocol`)
- **Variables**: snake_case with `_` prefix for unused (e.g., `_unused_var`)
- **Avoid similar names**: `encoder` vs `encoded`, `cursor` vs `cur`

### Error Handling
- Use `thiserror` for error enum definitions
- Implement `From` for error conversions
- Use `?` operator for propagating errors
- Return `RedisResult<T>` where `T` is the success type
- Errors should be descriptive with context

### Types
- Use `i64` for Redis integers (Redis protocol uses 64-bit)
- Use `Bytes` from `bytes` crate for zero-copy data
- Use `Arc<RwLock<T>>` for shared mutable state
- Use `Option<T>` for nullable Redis values
- Use `Vec<T>` for arrays from Redis
- Use `Self` instead of type name in enum variants (e.g., `Array(Vec<Self>)`)

### Async/Tokio
- All async functions should be `async`
- Use `tokio::sync` primitives (Mutex, RwLock, mpsc, oneshot)
- Prefer `tokio::time::timeout` for operation timeouts
- Use `async-trait` for trait method async support

### Protocol Implementation (resp2.rs)
- Encoder: Use `RespEncoder` instance with `encode()` and `encode_command()` methods
- Decoder: Use `RespDecoder::decode(&mut Cursor<&[u8]>)` static method
- Buffer: Use `BytesMut` for encoding buffer, `Bytes` for output
- Never use static methods on encoder/decoder - create instance first

### Modules Structure
- `src/lib.rs`: Main library exports
- `src/client.rs`: High-level Client API
- `src/pool.rs`: Connection pooling (merged pool + pool_optimized)
- `src/protocol/`: RESP2/RESP3 protocol implementation
- `src/commands/`: Command builders (use trait pattern)
- `src/cluster.rs`: Cluster detection and redirect handling
- `tests/`: Integration tests (require Redis)
- `benches/`: Benchmarks

### Testing Guidelines
- Unit tests in source files with `#[cfg(test)]`
- Integration tests in `tests/` directory
- Use `tokio::test` for async tests
- Mock Redis responses in unit tests
- Use testcontainers for integration tests

### Feature Flags
- `internal-optimizations`: For benchmarks/demos only, gated behind feature flag
- Do not use unstable features
- All public API must be stable

### Documentation
- Document all public items
- Use `///` doc comments
- Include examples in doc comments where helpful
- Update docs/ directory for user-facing documentation

### Clippy Compliance (CRITICAL)
- ALWAYS run `cargo clippy --all-targets --all-features -- -D warnings` before committing
- CI enforces `-D warnings` on ALL targets (lib + tests + benchmarks + examples)
- Common issues:
  - `similar_names`: Don't use `encoder` and `encoded` in same scope
  - `use_self`: Use `Self` instead of type name in enum variants
  - `self_only_used_in_recursion`: Use standalone function for recursive helpers

### Pre-Commit Checklist
- [ ] `cargo fmt` runs clean
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes with 0 errors
- [ ] `cargo test --lib` passes
- [ ] No `TODO` or `FIXME` comments without documentation
- [ ] No `#![allow(...)]` unless intentional and documented
