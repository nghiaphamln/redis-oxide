# Contributing to redis-mux

Thank you for your interest in contributing to redis-mux! This document provides guidelines for contributing to the project.

## Development Setup

### Prerequisites

- Rust 1.80 or later
- Redis server for testing (6.0+)

### Clone and Build

```bash
git clone https://github.com/yourusername/redis-oxide.git
cd redis-oxide
cargo build
```

### Run Tests

```bash
# Run unit tests
cargo test --lib

# Run integration tests (requires Redis server)
cargo test --test '*'

# Run all tests
cargo test
```

### Code Quality

Before submitting a PR, ensure:

```bash
# Format code
cargo fmt --all

# Run clippy
cargo clippy --workspace --all-targets --all-features

# Build release
cargo build --release
```

## Code Guidelines

1. **Code Style**: Follow Rust standard formatting (use `cargo fmt`)
2. **Documentation**: Add doc comments for all public APIs
3. **Tests**: Write tests for new functionality
4. **Error Handling**: Use the `RedisError` type for all errors
5. **Async**: Use Tokio runtime for async operations

## Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Add tests for your changes
5. Run all tests and checks
6. Commit your changes (`git commit -m 'Add amazing feature'`)
7. Push to the branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

## Testing Guidelines

- Write unit tests for individual components
- Write integration tests for end-to-end functionality
- Use `testcontainers` for integration tests requiring Redis
- Ensure tests are deterministic and don't rely on external state

## Documentation

- Update README.md if adding new features
- Add examples in `examples/` for significant new functionality
- Update CHANGELOG.md following Keep a Changelog format

## Questions?

Feel free to open an issue for questions or discussions.

Thank you for contributing!
