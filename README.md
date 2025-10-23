# redis-oxide

A high-performance, async Redis client for Rust with a focus on correctness, high-level abstractions, and ease of use.

[![Crates.io](https://img.shields.io/crates/v/redis-oxide.svg)](https://crates.io/crates/redis-oxide)
[![Docs.rs](https://docs.rs/redis-oxide/badge.svg)](https://docs.rs/redis-oxide)
[![Build Status](https://github.com/nghiaphamln/redis-oxide/workflows/ci/badge.svg)](https://github.com/nghiaphamln/redis-oxide/actions)

## Features

- **Async Support**: Built on top of `tokio` for asynchronous operations.
- **Connection Pooling**: Efficient connection pooling to manage multiple Redis connections.
- **Multiplexing**: Support for multiplexing multiple requests over a single connection.
- **Cluster Support**: Automatic cluster slot management and redirection handling.
- **Type-Safe Commands**: A type-safe command API to prevent errors at compile time.
- **Error Handling**: Comprehensive error handling with a clear and concise API.
- **High Performance**: Optimized for high performance and low overhead.

## Getting Started

Add `redis-oxide` to your `Cargo.toml`:

```toml
[dependencies]
redis-oxide = "0.2.0"
```

## Basic Usage

```rust
use redis_oxide::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::new("redis://127.0.0.1:6379")?;
    let client = Client::connect(config).await?;

    client.set("key", "value").await?;
    let value: String = client.get("key").await?;

    println!("Got value: {}", value);

    Ok(())
}
```

## Documentation

For more detailed information, please see the [documentation](https://docs.rs/redis-oxide).

## Contributing

Contributions are welcome! Please see the [contributing guide](CONTRIBUTING.md) for more details.

## License

This project is licensed under either of the following, at your option:

- Apache License, Version 2.0, ([LICENSE-APACHE](https://github.com/nghiaphamln/redis-oxide/blob/main/LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](https://github.com/nghiaphamln/redis-oxide/blob/main/LICENSE-MIT) or http://opensource.org/licenses/MIT)
