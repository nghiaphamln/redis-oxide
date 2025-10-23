# Documentation Guide for redis-oxide

This document explains how to work with and contribute to the documentation for `redis-oxide`.

## 📚 Documentation Structure

### 1. Main Documentation

- **README.md**: Comprehensive user guide with examples and features
- **API Documentation**: Generated from doc comments in source code
- **CONTRIBUTING.md**: Guidelines for contributors

### 2. Source Code Documentation

All public APIs are documented with:
- Doc comments (`///` and `//!`)
- Usage examples in doc comments
- Error documentation where applicable
- Type safety explanations

### 3. Examples

Located in `examples/` directory:
- `basic_usage.rs`: Basic Redis operations
- `cluster_usage.rs`: Redis Cluster specific features
- `pool_strategies.rs`: Connection pooling strategies

## 🛠️ Building Documentation

### Generate API Documentation

```bash
# Generate documentation
cargo doc --no-deps

# Generate and open in browser
cargo doc --no-deps --open

# Generate with private items (for development)
cargo doc --no-deps --document-private-items --open
```

### Test Documentation

```bash
# Run doc tests to ensure examples work
cargo test --doc

# Run specific doc tests
cargo test --doc --package redis-oxide
```

### Check Documentation Quality

```bash
# Check for missing documentation
cargo clippy -- -W missing_docs

# Check for broken links (requires cargo-deadlinks)
cargo deadlinks --check-http
```

## 📝 Writing Documentation

### Doc Comments Style

```rust
/// Brief description of the function/struct
///
/// More detailed explanation if needed. This can span multiple
/// paragraphs and include markdown formatting.
///
/// # Arguments
///
/// * `param1` - Description of parameter 1
/// * `param2` - Description of parameter 2
///
/// # Returns
///
/// Description of what this function returns
///
/// # Errors
///
/// This function will return an error if:
/// - Condition 1 occurs
/// - Condition 2 occurs
///
/// # Examples
///
/// ```
/// use redis_oxide::{Client, ConnectionConfig};
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let config = ConnectionConfig::new("redis://localhost:6379");
/// let client = Client::connect(config).await?;
/// # Ok(())
/// # }
/// ```
///
/// # Panics
///
/// This function panics if condition X occurs.
pub fn example_function(param1: &str, param2: i32) -> Result<String, Error> {
    // Implementation
}
```

### Module Documentation

```rust
//! Module description
//!
//! This module provides functionality for X, Y, and Z.
//!
//! # Examples
//!
//! ```
//! use redis_oxide::module_name::SomeType;
//!
//! let instance = SomeType::new();
//! ```
```

### Documentation Sections

Use these standard sections in order:

1. **Brief description** (first line)
2. **Detailed description** (if needed)
3. **Arguments** (for functions)
4. **Returns** (for functions)
5. **Errors** (for fallible functions)
6. **Examples** (always include when possible)
7. **Panics** (if function can panic)
8. **Safety** (for unsafe functions)

### Example Guidelines

- Always use `# #[tokio::main]` and `# async fn main()` for async examples
- Use `# Ok(())` and `# }` to complete examples
- Use `no_run` attribute for examples that require external dependencies
- Include error handling in examples
- Show realistic usage patterns

## 🔍 Documentation Features

### Cargo.toml Configuration

The project is configured for optimal documentation generation:

```toml
[package]
documentation = "https://docs.rs/redis-oxide"
readme = "../README.md"
include = [
    "src/**/*",
    "benches/**/*", 
    "tests/**/*",
    "examples/**/*",
    "Cargo.toml",
    "../README.md",
    "../LICENSE-MIT",
    "../LICENSE-APACHE"
]

[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]
targets = ["x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc", "x86_64-apple-darwin"]
```

### Features

- **Cross-platform targets**: Documentation built for Linux, Windows, and macOS
- **Feature flags**: All features enabled during documentation build
- **README inclusion**: README.md is included in the published crate
- **License files**: Both MIT and Apache licenses included

## 📋 Documentation Checklist

Before publishing or submitting PR:

- [ ] All public APIs have doc comments
- [ ] Examples compile and work (`cargo test --doc`)
- [ ] README.md is up to date
- [ ] No clippy warnings about missing docs
- [ ] Examples in `examples/` directory work
- [ ] Cross-references between docs are correct
- [ ] Error types are documented
- [ ] Performance characteristics mentioned where relevant

## 🚀 Publishing Documentation

### docs.rs

Documentation is automatically published to [docs.rs](https://docs.rs/redis-oxide) when:
- A new version is published to crates.io
- The build succeeds on docs.rs infrastructure

### Local Preview

To preview how documentation will look on docs.rs:

```bash
# Set the docs.rs environment variable
DOCS_RS=1 cargo doc --no-deps --open
```

## 🤝 Contributing to Documentation

### Types of Documentation Contributions

1. **Fix typos and grammar**
2. **Improve examples**
3. **Add missing documentation**
4. **Update outdated information**
5. **Improve clarity and readability**

### Documentation Review Process

1. Check that examples compile: `cargo test --doc`
2. Verify links work
3. Ensure consistent style and formatting
4. Test examples manually if possible
5. Check for completeness and accuracy

## 📚 Resources

- [Rust Documentation Guidelines](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html)
- [docs.rs documentation](https://docs.rs/about)
- [The rustdoc book](https://doc.rust-lang.org/rustdoc/)
- [RFC 1574 - More API Documentation Conventions](https://github.com/rust-lang/rfcs/blob/master/text/1574-more-api-documentation-conventions.md)
