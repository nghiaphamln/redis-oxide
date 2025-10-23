# redis-oxide Crate Documentation

This document provides a comprehensive overview of the `redis-oxide` crate, its features, and how to use it.

## Overview

`redis-oxide` is a high-performance, async Redis client for Rust designed for correctness, high-level abstractions, and ease of use. It provides a rich feature set, including:

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

    // Set a key
    client.set("key", "value").await?;

    // Get a key
    let value: String = client.get("key").await?;
    println!("Got value: {}", value);

    // Delete a key
    client.del("key").await?;

    Ok(())
}
```

## Type-Safe Commands

`redis-oxide` provides a set of type-safe command builders for common Redis commands.

### SET

The `SET` command can be used to set a key with a value and optional expiration.

```rust
use redis_oxide::{Client, ConnectionConfig, SetOptions};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::new("redis://127.0.0.1:6379")?;
    let client = Client::connect(config).await?;

    // Set a key with a 10-second expiration
    let options = SetOptions::new().ex(Duration::from_secs(10));
    client.set_with_options("key", "value", options).await?;

    Ok(())
}
```

### GET

The `GET` command can be used to get the value of a key.

```rust
use redis_oxide::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::new("redis://127.0.0.1:6379")?;
    let client = Client::connect(config).await?;

    let value: Option<String> = client.get("key").await?;

    match value {
        Some(v) => println!("Got value: {}", v),
        None => println!("Key not found"),
    }

    Ok(())
}
```

### DEL

The `DEL` command can be used to delete one or more keys.

```rust
use redis_oxide::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::new("redis://127.0.0.1:6379")?;
    let client = Client::connect(config).await?;

    client.del("key1").await?;
    client.del_slice(&["key2", "key3"]).await?;

    Ok(())
}
```

### EXPIRE

The `EXPIRE` command can be used to set a timeout on a key.

```rust
use redis_oxide::{Client, ConnectionConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::new("redis://127.0.0.1:6379")?;
    let client = Client::connect(config).await?;

    client.expire("key", Duration::from_secs(10)).await?;

    Ok(())
}
```

### INCR

The `INCR` command can be used to increment the integer value of a key by one.

```rust
use redis_oxide::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::new("redis://127.0.0.1:6379")?;
    let client = Client::connect(config).await?;

    let value: i64 = client.incr("counter").await?;
    println!("Counter: {}", value);

    Ok(())
}
```

## Cluster Usage

`redis-oxide` provides automatic cluster slot management and redirection handling.

```rust
use redis_oxide::{Client, ConnectionConfig, TopologyMode};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::builder()
        .add_endpoint("redis://127.0.0.1:7000")
        .add_endpoint("redis://127.0.0.1:7001")
        .add_endpoint("redis://127.0.0.1:7002")
        .topology_mode(TopologyMode::Cluster)
        .build()?;

    let client = Client::connect(config).await?;

    client.set("key", "value").await?;
    let value: String = client.get("key").await?;

    println!("Got value: {}", value);

    Ok(())
}
```

## Connection Pooling

`redis-oxide` supports two connection pooling strategies:

- **Multiplexed**: A single connection is shared for all requests. This is the default and is suitable for most use cases.
- **Pool**: A traditional connection pool with a configurable number of connections.

### Multiplexed

```rust
use redis_oxide::{Client, ConnectionConfig, PoolConfig, PoolStrategy};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::builder()
        .add_endpoint("redis://127.0.0.1:6379")
        .pool_config(PoolConfig::new(PoolStrategy::Multiplexed))
        .build()?;

    let client = Client::connect(config).await?;

    // ...

    Ok(())
}
```

### Pool

```rust
use redis_oxide::{Client, ConnectionConfig, PoolConfig, PoolStrategy};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::builder()
        .add_endpoint("redis://127.0.0.1:6379")
        .pool_config(PoolConfig::new(PoolStrategy::Pool(2, 10)))
        .build()?;

    let client = Client::connect(config).await?;

    // ...

    Ok(())
}
```

## Error Handling

`redis-oxide` provides a comprehensive error handling system. The `RedisError` enum represents all possible errors that can occur.

```rust
use redis_oxide::{Client, ConnectionConfig, RedisError};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::new("redis://127.0.0.1:6379")?;
    let client = Client::connect(config).await?;

    match client.get("non-existent-key").await {
        Ok(value) => println!("Got value: {:?}", value),
        Err(RedisError::NotFound) => println!("Key not found"),
        Err(e) => println!("An error occurred: {}", e),
    }

    Ok(())
}
```

## Roadmap

- **Pipeline support**: Batch commands
- **Pub/Sub**: Publish/Subscribe
- **Transactions**: MULTI/EXEC
- **More commands**: Hash, List, Set, Sorted Set
- **Lua scripting**: EVAL/EVALSHA
- **Redis Streams**: Stream operations
- **RESP3 protocol**: Latest protocol version
- **Sentinel support**: High availability

## Contributing

Contributions are welcome! Please see the [contributing guide](CONTRIBUTING.md) for more details.

## License

This project is licensed under either of the following, at your option:

- Apache License, Version 2.0, ([LICENSE-APACHE](https://github.com/nghiaphamln/redis-oxide/blob/main/LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](https://github.com/nghiaphamln/redis-oxide/blob/main/LICENSE-MIT) or http://opensource.org/licenses/MIT)