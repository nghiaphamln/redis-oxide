# redis-oxide

[![Crates.io](https://img.shields.io/crates/v/redis-oxide.svg)](https://crates.io/crates/redis-oxide)
[![Docs.rs](https://docs.rs/redis-oxide/badge.svg)](https://docs.rs/redis-oxide)
[![Build Status](https://github.com/nghiaphamln/redis-oxide/workflows/ci/badge.svg)](https://github.com/nghiaphamln/redis-oxide/actions)
[![License: MIT OR Apache-2.0](https://img.shields.io/crates/l/redis-oxide.svg)](https://github.com/nghiaphamln/redis-oxide#license)

A high-performance, async Redis client for Rust, inspired by StackExchange.Redis for .NET. Features automatic cluster detection, MOVED/ASK redirect handling, and flexible connection strategies.

## 🚀 Features

- **Automatic topology detection**: Auto-recognizes Standalone Redis or Redis Cluster
- **MOVED/ASK redirect handling**: Automatically handles slot migrations in cluster mode
- **Flexible connection strategies**: Supports both Multiplexed connections and Connection Pools
- **Type-safe command builders**: Safe API with builder pattern
- **Async/await**: Fully asynchronous with Tokio runtime
- **Automatic reconnection**: Reconnects with exponential backoff
- **Comprehensive error handling**: Detailed and clear error types
- **High test coverage**: Extensive unit and integration tests
- **Cross-platform support**: Works on Linux, macOS, and Windows

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
redis-oxide = "0.2.0"
tokio = { version = "1.0", features = ["full"] }
```

## 🛠️ Quick Start

### Basic Connection (Standalone)

```rust
use redis_oxide::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create configuration
    let config = ConnectionConfig::new("redis://localhost:6379");
    
    // Connect (automatically detects topology)
    let client = Client::connect(config).await?;
    
    // SET and GET
    client.set("mykey", "Hello, Redis!").await?;
    if let Some(value) = client.get("mykey").await? {
        println!("Value: {}", value);
    }
    
    Ok(())
}
```

### Redis Cluster Connection

```rust
use redis_oxide::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Provide multiple seed nodes
    let config = ConnectionConfig::new(
        "redis://node1:7000,node2:7001,node3:7002"
    );
    
    let client = Client::connect(config).await?;
    
    // Client automatically handles MOVED redirects
    client.set("key", "value").await?;
    
    Ok(())
}
```

## 🎯 Supported Operations

### String Operations

```rust
use redis_oxide::{Client, ConnectionConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::new("redis://localhost:6379");
    let client = Client::connect(config).await?;

    // GET
    let value: Option<String> = client.get("key").await?;

    // SET
    client.set("key", "value").await?;

    // SET with expiration
    client.set_ex("key", "value", Duration::from_secs(60)).await?;

    // SET NX (only if key doesn't exist)
    let set: bool = client.set_nx("key", "value").await?;

    // DELETE
    let deleted: i64 = client.del(vec!["key1".to_string(), "key2".to_string()]).await?;

    // EXISTS
    let exists: i64 = client.exists(vec!["key".to_string()]).await?;

    // EXPIRE
    client.expire("key", Duration::from_secs(60)).await?;

    // TTL
    let ttl: Option<i64> = client.ttl("key").await?;

    // INCR/DECR
    let new_value: i64 = client.incr("counter").await?;
    let new_value: i64 = client.decr("counter").await?;

    // INCRBY/DECRBY
    let new_value: i64 = client.incr_by("counter", 10).await?;
    let new_value: i64 = client.decr_by("counter", 5).await?;

    Ok(())
}
```

### Connection Pool Configuration

```rust
use redis_oxide::{Client, ConnectionConfig, PoolConfig, PoolStrategy};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = ConnectionConfig::new("redis://localhost:6379");
    
    // Configure connection pool
    config.pool = PoolConfig {
        strategy: PoolStrategy::Pool,
        max_size: 20,                      // Max 20 connections
        min_idle: 5,                       // Keep at least 5 idle connections
        connection_timeout: Duration::from_secs(5),
    };
    
    let client = Client::connect(config).await?;
    Ok(())
}
```

### Multiplexed Connection (Default)

```rust
use redis_oxide::{Client, ConnectionConfig, PoolConfig, PoolStrategy};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = ConnectionConfig::new("redis://localhost:6379");
    
    // Use multiplexed connection (recommended for most use cases)
    config.pool = PoolConfig {
        strategy: PoolStrategy::Multiplexed,
        ..Default::default()
    };
    
    let client = Client::connect(config).await?;
    Ok(())
}
```

### Transactions

```rust
use redis_oxide::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::new("redis://localhost:6379");
    let client = Client::connect(config).await?;

    // Start a transaction
    let mut transaction = client.transaction().await?;

    // Add commands to the transaction (no await for adding commands)
    transaction.set("key1", "value1");
    transaction.set("key2", "value2");
    transaction.incr("counter");

    // Execute the transaction
    let results = transaction.exec().await?;
    println!("Transaction results: {:?}", results);

    Ok(())
}
```

### Pipelines

```rust
use redis_oxide::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::new("redis://localhost:6379");
    let client = Client::connect(config).await?;

    // Create a pipeline
    let mut pipeline = client.pipeline();

    // Add commands to the pipeline (no await for adding commands)
    pipeline.set("key1", "value1");
    pipeline.set("key2", "value2");
    pipeline.get("key1");
    pipeline.incr("counter");

    // Execute all commands at once
    let results = pipeline.execute().await?;
    println!("Pipeline results: {:?}", results);

    Ok(())
}
```

### Hash Operations

```rust
use redis_oxide::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::new("redis://localhost:6379");
    let client = Client::connect(config).await?;

    // HSET
    client.hset("myhash", "field1", "value1").await?;
    client.hset("myhash", "field2", "value2").await?;

    // HGET
    if let Some(value) = client.hget("myhash", "field1").await? {
        println!("field1: {}", value);
    }

    // HGETALL
    let all_fields = client.hgetall("myhash").await?;
    println!("All hash fields: {:?}", all_fields);

    // HMGET
    let values = client.hmget("myhash", vec!["field1".to_string(), "field2".to_string()]).await?;
    println!("Multiple fields: {:?}", values);

    Ok(())
}
```

## ⚙️ Configuration Options

### Connection Configuration

```rust
use redis_oxide::{ConnectionConfig, TopologyMode};
use std::time::Duration;

let config = ConnectionConfig::new("redis://localhost:6379")
    .with_password("secret")           // Password (optional)
    .with_database(0)                  // Database number
    .with_connect_timeout(Duration::from_secs(5))
    .with_operation_timeout(Duration::from_secs(30))
    .with_topology_mode(TopologyMode::Auto) // Auto, Standalone, or Cluster
    .with_max_redirects(3);            // Max retries for cluster redirects
```

### Advanced Configuration

```rust
use redis_oxide::{ConnectionConfig, ProtocolVersion};
use std::time::Duration;

let config = ConnectionConfig::new("redis://localhost:6379")
    .with_protocol_version(ProtocolVersion::Resp3)  // Use RESP3 protocol
    .with_tcp_keepalive(Some(Duration::from_secs(60)))  // Enable keepalive
    .with_reconnect_enabled(true)
    .with_reconnect_initial_delay(Duration::from_millis(100))
    .with_reconnect_max_delay(Duration::from_secs(30));
```

## 🧪 Testing

To run the tests, you'll need a Redis server running on `localhost:6379`:

```bash
# Run all tests
cargo test

# Run integration tests specifically
cargo test --test integration_tests

# Run tests with Redis server
docker run --rm -p 6379:6379 redis:7
cargo test
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for more details.

### Development Setup

1. Fork and clone the repository
2. Install Rust: https://rustup.rs/
3. Run tests to confirm everything works:
   ```bash
   docker run --rm -p 6379:6379 redis:7
   cargo test
   ```

## 📄 License

This project is licensed under either of the following, at your option:

- Apache License, Version 2.0, ([LICENSE-APACHE](https://github.com/nghiaphamln/redis-oxide/blob/main/LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](https://github.com/nghiaphamln/redis-oxide/blob/main/LICENSE-MIT) or http://opensource.org/licenses/MIT)

## 📚 Documentation

For more detailed information, please see the [documentation](https://docs.rs/redis-oxide).

## 🐛 Issues and Feature Requests

Please report bugs or suggest features in the [GitHub Issues](https://github.com/nghiaphamln/redis-oxide/issues).