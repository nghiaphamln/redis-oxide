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

## Project Structure

```
redis-mux/
├── Cargo.toml                          # Workspace manifest
├── README.md                           # Main documentation
├── CHANGELOG.md                        # Change log
├── CONTRIBUTING.md                     # Contribution guidelines
├── LICENSE-MIT / LICENSE-APACHE       # Dual licensing
│
├── redis-oxide-core/                   # Core library
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                      # Module exports
│       ├── error.rs                    # RedisError with MOVED parsing
│       ├── config.rs                   # ConnectionConfig, PoolConfig
│       ├── types.rs                    # RedisValue, NodeInfo, SlotRange
│       └── value.rs                    # RespValue types
│
├── redis-oxide/                        # Main library
│   ├── Cargo.toml
│   ├── benches/
│   │   └── protocol_bench.rs           # Protocol benchmarks
│   ├── tests/
│   │   └── integration_tests.rs        # Integration tests
│   └── src/
│       ├── lib.rs                      # Public API
│       ├── protocol.rs                 # RESP2 encoder/decoder
│       ├── connection.rs               # Connection management
│       ├── cluster.rs                  # Cluster support
│       ├── pool.rs                     # Connection pooling
│       ├── client.rs                   # High-level Client API
│       └── commands/
│           └── mod.rs                  # Command builders
│
├── examples/                           # Usage examples
│   ├── basic_usage.rs                  # Basic operations
│   ├── cluster_usage.rs                # Cluster operations
│   └── pool_strategies.rs              # Pool comparison
│
└── .github/
    └── workflows/
        └── ci.yml                      # CI/CD pipeline
```

## Main Components

### Core Types (`redis-oxide-core`)

- **Error Handling**: `RedisError` with variants for all error cases, including `MOVED` and `ASK` redirection parsing.
- **Configuration**: `ConnectionConfig`, `PoolConfig`, and `TopologyMode` for configuring the client.
- **Value Types**: `RespValue`, `RedisValue`, `NodeInfo`, and `SlotRange` for representing Redis data and cluster topology.

### Protocol Layer (`protocol.rs`)

- **RespEncoder**: Encodes `RespValue` to bytes.
- **RespDecoder**: Decodes RESP from a buffer.

### Connection Management (`connection.rs`)

- **RedisConnection**: Manages a TCP connection to a Redis server.
- **ConnectionManager**: Detects the Redis topology (standalone or cluster) and creates connections.

### Cluster Support (`cluster.rs`)

- **Slot Calculation**: Calculates the cluster slot for a given key.
- **ClusterTopology**: Manages the slot-to-node mapping.
- **RedirectHandler**: Handles `MOVED` and `ASK` redirections.

### Connection Pooling (`pool.rs`)

- **MultiplexedPool**: A single connection shared via an mpsc channel.
- **ConnectionPool**: A traditional connection pool with a configurable number of connections.

### Command Builders (`commands/`)

- **Type-Safe Commands**: A set of type-safe command builders for common Redis commands.

### Client API (`client.rs`)

- **Client**: The main entry point for interacting with Redis.

## Getting Started

Add `redis-oxide` to your `Cargo.toml`:

```toml
[dependencies]
redis-oxide = "0.1.0"
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

## Cluster Usage

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

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
