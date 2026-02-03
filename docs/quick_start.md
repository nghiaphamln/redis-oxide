# Quick Start

This guide will help you get started with redis-oxide quickly.

## Installation

Add redis-oxide to your `Cargo.toml`:

```toml
[dependencies]
redis-oxide = "0.2"
```

## Basic Connection

```rust
use redis_oxide::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Redis
    let config = ConnectionConfig::new("redis://localhost:6379");
    let client = Client::connect(config).await?;

    // Verify connection
    let pong: String = client.ping().await?;
    assert_eq!(pong, "PONG");

    Ok(())
}
```

## String Operations

```rust
// Set a key
client.set("mykey", "myvalue").await?;

// Get a key
let value: Option<String> = client.get("mykey").await?;
assert_eq!(value, Some("myvalue".to_string()));

// Delete a key
let deleted = client.del("mykey").await?;
assert_eq!(deleted, 1);

// Check if key exists
let exists = client.exists("mykey").await?;
assert_eq!(exists, 0);
```

## Counter Operations

```rust
// Set a counter
client.set("counter", "0").await?;

// Increment by 1
let value: i64 = client.incr("counter", 1).await?;
assert_eq!(value, 1);

// Increment by 10
let value: i64 = client.incr("counter", 10).await?;
assert_eq!(value, 11);

// Decrement by 1
let value: i64 = client.decr("counter", 1).await?;
assert_eq!(value, 10);
```

## Expiration

```rust
// Set a key with expiration (seconds)
client.set_ex("temp_key", "temp_value", 60).await?;

// Set key only if not exists (NX)
client.set_nx("unique_key", "unique_value").await?;

// Set key only if exists (XX)
client.set_xx("existing_key", "new_value").await?;

// Set expiration on existing key
client.expire("mykey", 3600).await?;

// Get time to live
let ttl: i64 = client.ttl("mykey").await?;
```

## Running the Examples

The repository includes example applications:

```bash
# Basic usage example
cargo run --example basic_usage

# Cluster mode example
cargo run --example cluster_usage

# Pool strategies example
cargo run --example pool_strategies
```

## Next Steps

- Learn about [Connection Configuration](configuration.md)
- Explore available [Commands](commands.md)
- Understand [Connection Pooling](pooling.md)
