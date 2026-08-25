# redis-oxide

[![Crates.io](https://img.shields.io/crates/v/redis-oxide.svg)](https://crates.io/crates/redis-oxide)
[![Docs.rs](https://docs.rs/redis-oxide/badge.svg)](https://docs.rs/redis-oxide)
[![License: MIT](https://img.shields.io/crates/l/redis-oxide.svg)](https://github.com/nghiaphamln/redis-oxide#license)

`redis-oxide` is an async Redis client for Rust. It provides topology detection,
Redis Cluster support, Sentinel support, connection pooling, RESP2/RESP3
protocol handling, Pub/Sub, Streams, Lua scripting, transactions, and pipelines.

The crate is designed for Tokio applications that need a practical Redis client
with a typed API and predictable behavior across standalone and clustered Redis
deployments.

## Installation

```toml
[dependencies]
redis-oxide = "0.3.0-alpha.1"
tokio = { version = "1", features = ["full"] }
```

This is a breaking alpha release. Read the [0.3 migration guide](docs/migration-0.3.md)
before upgrading, or use `cargo add redis-oxide` for the latest published release.

## Quick Start

```rust
use redis_oxide::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::new("redis://localhost:6379");
    let client = Client::connect(config).await?;

    client.set("greeting", "hello").await?;

    if let Some(value) = client.get("greeting").await? {
        println!("{value}");
    }

    Ok(())
}
```

## Core Operations

```rust
use std::collections::HashMap;
use std::time::Duration;

// Assume `client` is a connected redis_oxide::Client.
client.set("key", "value").await?;
client.set_ex("session", "active", Duration::from_secs(60)).await?;

let value: Option<String> = client.get("key").await?;
let ttl: Option<i64> = client.ttl("session").await?;

client.hset("user:1", "name", "Nghia").await?;
let fields: HashMap<String, String> = client.hgetall("user:1").await?;

client.lpush("jobs", vec!["job-1".to_string(), "job-2".to_string()]).await?;
let jobs: Vec<String> = client.lrange("jobs", 0, -1).await?;

client.sadd("tags", vec!["rust".to_string(), "redis".to_string()]).await?;
let tags = client.smembers("tags").await?;

let mut scores = HashMap::new();
scores.insert("alice".to_string(), 10.0);
scores.insert("bob".to_string(), 12.0);
client.zadd("leaderboard", scores).await?;
```

`ttl` preserves Redis sentinel values. A missing key returns `Some(-2)` and a key
without expiration returns `Some(-1)`.

## Advanced Usage

### Pipelines

```rust
let mut pipeline = client.pipeline();
pipeline.set("key:1", "value:1");
pipeline.set("key:2", "value:2");
pipeline.get("key:1");

let results = pipeline.execute().await?;
```

Redis server errors inside a pipeline are returned as per-command
`RespValue::Error` entries. Transport and connection failures still fail the
whole pipeline.

### Transactions

```rust
let mut transaction = client.transaction().await?;
transaction.watch(vec!["balance".to_string()]).await?;
transaction.get("balance");
transaction.set("balance", "120");

let results = transaction.exec().await?;
```

### Lua Scripts

```rust
use redis_oxide::Script;

let script = Script::new(
    "return KEYS[1] .. ':' .. ARGV[1]"
);

let result: String = script.execute(
    &client,
    vec!["user".to_string()],
    vec!["42".to_string()],
).await?;
```

`Script::execute` tries `EVALSHA` first and falls back to `EVAL` when Redis
returns `NOSCRIPT`.

### Pub/Sub

```rust
client.publish("events", "created").await?;

let mut subscriber = client.subscriber().await?;
subscriber.subscribe(vec!["events".to_string()]).await?;

if let Some(message) = subscriber.next_message().await? {
    println!("{}: {}", message.channel, message.payload);
}
```

### Streams

```rust
use std::collections::HashMap;

let mut fields = HashMap::new();
fields.insert("event".to_string(), "created".to_string());

let id = client.xadd("events", "*", fields).await?;
let messages = client
    .xread(vec![("events".to_string(), "0".to_string())], Some(10), None)
    .await?;
```

## Configuration

```rust
use redis_oxide::{ConnectionConfig, PoolConfig, PoolStrategy, ProtocolVersion};
use std::time::Duration;

let config = ConnectionConfig::new("redis://localhost:6379")
    .with_operation_timeout(Duration::from_secs(10))
    .with_protocol_version(ProtocolVersion::Resp2)
    .with_pool_config(PoolConfig {
        strategy: PoolStrategy::Multiplexed,
        max_size: 10,
        min_idle: 2,
        connection_timeout: Duration::from_secs(5),
    });
```

Use `ConnectionConfig::new("redis://host1:6379,host2:6379")` with cluster
topology detection enabled for Redis Cluster deployments. Sentinel configuration
is available through `SentinelConfig`.

## Development

Required toolchain:

- Rust 1.82.0 or newer
- Redis available on `localhost:6379` for integration tests

Quality gates:

```bash
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo doc --workspace --no-deps --document-private-items
cargo audit
cargo deny check
```

## Documentation

- [Getting started](https://github.com/nghiaphamln/redis-oxide/blob/main/docs/getting-started.md)
- [Architecture](https://github.com/nghiaphamln/redis-oxide/blob/main/docs/architecture.md)
- [Performance](https://github.com/nghiaphamln/redis-oxide/blob/main/docs/performance.md)
- [Troubleshooting](https://github.com/nghiaphamln/redis-oxide/blob/main/docs/troubleshooting.md)
- [0.3 migration guide](https://github.com/nghiaphamln/redis-oxide/blob/main/docs/migration-0.3.md)
- [Changelog](https://github.com/nghiaphamln/redis-oxide/blob/main/CHANGELOG.md)
- [Contributing](https://github.com/nghiaphamln/redis-oxide/blob/main/CONTRIBUTING.md)
- [API documentation](https://docs.rs/redis-oxide)

## Compatibility

- Rust: 1.82.0 or newer
- Redis: 6.0 or newer
- Platforms: Linux, macOS, Windows
- Runtime: Tokio

## License

This project is licensed under the MIT License. See
[LICENSE-MIT](https://github.com/nghiaphamln/redis-oxide/blob/main/LICENSE-MIT).
