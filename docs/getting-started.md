# Getting started

`redis-oxide` needs Rust 1.82 or newer, Tokio, and Redis 6.0 or newer.

Start a local Redis server:

```bash
docker run --rm -p 6379:6379 redis:7
redis-cli ping
```

Create a project and add dependencies:

```bash
cargo new redis-oxide-demo
cd redis-oxide-demo
```

```toml
[dependencies]
redis-oxide = "0.3.0-alpha.2"
tokio = { version = "1", features = ["full"] }
```

Use one client for normal application commands:

```rust
use redis_oxide::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::connect(ConnectionConfig::new("redis://localhost:6379")).await?;

    client.set("demo:key", "hello").await?;
    let value: Option<String> = client.get("demo:key").await?;
    println!("{value:?}");
    Ok(())
}
```

Run it with `cargo run`. For strings, hashes, lists, sets, sorted sets,
Streams, scripts, and transaction methods, use the generated
[API documentation](https://docs.rs/redis-oxide).

Next, read [deployment and topologies](deployment.md) for pooling, Cluster, or
Sentinel configuration.
