# Getting Started

This guide shows the smallest path from a new Rust project to a working
`redis-oxide` client.

## Requirements

- Rust 1.82.0 or newer
- Redis 6.0 or newer
- Tokio runtime

Start Redis locally:

```bash
docker run --rm -p 6379:6379 redis:7
```

Verify it:

```bash
redis-cli ping
```

## Create a Project

```bash
cargo new redis-oxide-demo
cd redis-oxide-demo
```

Add dependencies:

```toml
[dependencies]
redis-oxide = "0.2"
tokio = { version = "1", features = ["full"] }
```

Alternatively, run `cargo add redis-oxide` to use the latest published release.

## First Program

```rust
use redis_oxide::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::new("redis://localhost:6379");
    let client = Client::connect(config).await?;

    client.set("demo:key", "hello").await?;
    let value: Option<String> = client.get("demo:key").await?;

    println!("{value:?}");
    Ok(())
}
```

Run it:

```bash
cargo run
```

## Common Patterns

### Strings and Expiration

```rust
use std::time::Duration;

client.set("user:1:name", "Nghia").await?;
client.set_ex("session:1", "active", Duration::from_secs(300)).await?;

let name: Option<String> = client.get("user:1:name").await?;
let ttl: Option<i64> = client.ttl("session:1").await?;
```

Redis TTL sentinel values are preserved:

- `Some(-2)`: key does not exist
- `Some(-1)`: key exists without expiration
- `Some(n)`: key expires in `n` seconds

### Hashes

```rust
client.hset("user:1", "name", "Nghia").await?;
client.hset("user:1", "role", "backend").await?;

let user = client.hgetall("user:1").await?;
let name = client.hget("user:1", "name").await?;
```

### Lists

```rust
client
    .rpush("jobs", vec!["job-1".to_string(), "job-2".to_string()])
    .await?;

let jobs = client.lrange("jobs", 0, -1).await?;
```

### Pipelines

```rust
let mut pipeline = client.pipeline();
pipeline.set("a", "1");
pipeline.set("b", "2");
pipeline.get("a");

let results = pipeline.execute().await?;
```

### Transactions

```rust
let mut transaction = client.transaction().await?;
transaction.watch(vec!["counter".to_string()]).await?;
transaction.incr("counter");

let results = transaction.exec().await?;
```

### Pub/Sub

```rust
client.publish("events", "created").await?;

let mut subscriber = client.subscriber().await?;
subscriber.subscribe(vec!["events".to_string()]).await?;

if let Some(message) = subscriber.next_message().await? {
    println!("{} {}", message.channel, message.payload);
}
```

## Next Steps

- Read the [README](../README.md) for the feature overview.
- Read [Performance](performance.md) before choosing a connection strategy.
- Read [Troubleshooting](troubleshooting.md) when integration tests or local Redis fail.
