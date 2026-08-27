# redis-oxide

[![Crates.io](https://img.shields.io/crates/v/redis-oxide.svg)](https://crates.io/crates/redis-oxide)
[![Docs.rs](https://docs.rs/redis-oxide/badge.svg)](https://docs.rs/redis-oxide)
[![License: MIT](https://img.shields.io/crates/l/redis-oxide.svg)](https://github.com/nghiaphamln/redis-oxide/blob/main/LICENSE-MIT)

`redis-oxide` is an async Redis client for Tokio applications. It supports
standalone Redis, Redis Cluster, Redis Sentinel, pooling, pipelines,
transactions, Pub/Sub, Streams, Lua scripting, and RESP2 or RESP3.

## Install

```toml
[dependencies]
redis-oxide = "0.3.0-alpha.2"
tokio = { version = "1", features = ["full"] }
```

`0.3.0-alpha.2` is a breaking alpha release. Read the
[0.3 migration guide](https://github.com/nghiaphamln/redis-oxide/blob/main/docs/migration-0.3.md)
before upgrading from 0.2.

## Quick start

```rust
use redis_oxide::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::connect(ConnectionConfig::new("redis://localhost:6379")).await?;

    client.set("greeting", "hello").await?;
    println!("{:?}", client.get("greeting").await?);
    Ok(())
}
```

The full runnable version is
[`examples/basic_usage.rs`](https://github.com/nghiaphamln/redis-oxide/blob/main/redis-oxide/examples/basic_usage.rs).

## Compatibility and scope

- Rust 1.82 or newer.
- Redis 6.0 or newer.
- Tokio on Linux, macOS, and Windows.
- `redis://` seed URLs, including comma-separated Cluster seeds and bracketed
  IPv6 addresses.
- TLS is not implemented; `rediss://` URLs are rejected explicitly.

## Guides

- [Getting started](https://github.com/nghiaphamln/redis-oxide/blob/main/docs/getting-started.md)
- [Deployment and topologies](https://github.com/nghiaphamln/redis-oxide/blob/main/docs/deployment.md)
- [Troubleshooting](https://github.com/nghiaphamln/redis-oxide/blob/main/docs/troubleshooting.md)
- [Architecture](https://github.com/nghiaphamln/redis-oxide/blob/main/docs/architecture.md)
- [Migrating to 0.3](https://github.com/nghiaphamln/redis-oxide/blob/main/docs/migration-0.3.md)
- [API documentation](https://docs.rs/redis-oxide)
- [Changelog](https://github.com/nghiaphamln/redis-oxide/blob/main/CHANGELOG.md)
- [Contributing](https://github.com/nghiaphamln/redis-oxide/blob/main/CONTRIBUTING.md)

## Development

See [CONTRIBUTING.md](https://github.com/nghiaphamln/redis-oxide/blob/main/CONTRIBUTING.md)
for local setup and required checks.

## License

Licensed under the [MIT License](https://github.com/nghiaphamln/redis-oxide/blob/main/LICENSE-MIT).
