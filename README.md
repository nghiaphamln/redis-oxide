# redis-oxide

A high-performance Redis client library for Rust.

## Status

Documentation under development. For now, please see:
- [ARCHITECTURE.md](./ARCHITECTURE.md)
- [PERFORMANCE.md](./PERFORMANCE.md)

## Quick Start

```toml
[dependencies]
redis-oxide = "0.2"
```

```rust
use redis_oxide::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::new("redis://localhost:6379");
    let client = Client::connect(config).await?;

    client.set("key", "value").await?;
    let value: Option<String> = client.get("key").await?;

    println!("Value: {:?}", value);
    Ok(())
}
```

## Documentation

<https://docs.rs/redis-oxide>
