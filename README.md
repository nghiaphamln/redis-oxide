# redis-oxide

[![Crates.io](https://img.shields.io/crates/v/redis-oxide.svg)](https://crates.io/crates/redis-oxide)
[![Documentation](https://docs.rs/redis-oxide/badge.svg)](https://docs.rs/redis-oxide)
[![License](https://img.shields.io/crates/l/redis-oxide.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.82%2B-blue.svg)](https://www.rust-lang.org)

A high-performance async Redis client for Rust, similar to **StackExchange.Redis** for .NET. Automatically detects topology (Standalone or Cluster) and handles MOVED/ASK redirects transparently.

## ✨ Features

- 🚀 **Automatic topology detection**: Auto-recognizes Standalone Redis or Redis Cluster
- 🔄 **MOVED/ASK redirect handling**: Automatically handles slot migrations in cluster mode
- 🏊 **Flexible connection strategies**: Supports both Multiplexed connections and Connection Pools
- 🛡️ **Type-safe command builders**: Safe API with builder pattern
- ⚡ **Async/await**: Fully asynchronous with Tokio runtime
- 🔌 **Automatic reconnection**: Reconnects with exponential backoff
- 📊 **Comprehensive error handling**: Detailed and clear error types
- ✅ **High test coverage**: Extensive unit and integration tests

## 📋 Requirements

- **Rust**: 1.82 or later
- **Redis**: 6.0+ (Standalone) or Redis Cluster 6.0+

## 🚀 Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
redis-oxide = "0.1.0"
tokio = { version = "1", features = ["full"] }
```

## 📖 Quick Start

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

### Handling MOVED Redirects

When encountering a MOVED error (e.g., `MOVED 9916 10.90.6.213:6002`), the library will:

1. ✅ Parse the error message and extract slot number and target node
2. ✅ Automatically update slot mapping
3. ✅ Create new connection to target node if needed
4. ✅ Automatically retry the command (up to `max_redirects` times)

```rust
use redis_oxide::{Client, ConnectionConfig};

let config = ConnectionConfig::new("redis://cluster:7000")
    .with_max_redirects(5); // Allow up to 5 redirects

let client = Client::connect(config).await?;

// If encountering "MOVED 9916 10.90.6.213:6002", 
// client automatically retries command to 10.90.6.213:6002
let value = client.get("mykey").await?;
```

## 🎯 Supported Commands

### String Operations

```rust
// GET
let value: Option<String> = client.get("key").await?;

// SET
client.set("key", "value").await?;

// SET with expiration
use std::time::Duration;
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
```

## ⚙️ Configuration

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

### Pool Configuration

#### Multiplexed Connection (Default)

Uses a single connection shared between multiple tasks via mpsc channel. Suitable for most use cases.

```rust
use redis_oxide::{ConnectionConfig, PoolConfig, PoolStrategy};

let mut config = ConnectionConfig::new("redis://localhost:6379");
config.pool = PoolConfig {
    strategy: PoolStrategy::Multiplexed,
    ..Default::default()
};
```

#### Connection Pool

Uses multiple connections. Suitable for very high workload with many concurrent requests.

```rust
use redis_oxide::{PoolConfig, PoolStrategy};
use std::time::Duration;

let pool_config = PoolConfig {
    strategy: PoolStrategy::Pool,
    max_size: 20,                      // Max 20 connections
    min_idle: 5,                       // Keep at least 5 idle connections
    connection_timeout: Duration::from_secs(5),
};

let mut config = ConnectionConfig::new("redis://localhost:6379");
config.pool = pool_config;
```

## 🏗️ Architecture

```
redis-oxide/
└── redis-oxide/            # Main library
    ├── core/               # Core types, errors, config
    │   ├── error.rs        # RedisError with MOVED/ASK parsing
    │   ├── config.rs       # ConnectionConfig, PoolConfig
    │   ├── types.rs        # RedisValue, NodeInfo, SlotRange
    │   └── value.rs        # RespValue
    ├── protocol.rs         # RESP2 encoder/decoder
    ├── connection.rs       # TCP connection, topology detection
    ├── cluster.rs          # Slot calculation, redirect handling
    ├── pool.rs             # Multiplexed & Pool implementations
    ├── commands/           # Type-safe command builders
    └── client.rs           # High-level Client API
```

## 🔍 Comparison with StackExchange.Redis

| Feature | redis-oxide | StackExchange.Redis |
|---------|-------------|---------------------|
| Automatic topology detection | ✅ | ✅ |
| MOVED/ASK redirect handling | ✅ | ✅ |
| Multiplexed connection | ✅ | ✅ |
| Connection pooling | ✅ | ✅ |
| Async/await | ✅ (Tokio) | ✅ (Task) |
| Type-safe commands | ✅ (Builder pattern) | ✅ (Strongly typed) |
| Pipeline support | 🚧 Planned | ✅ |
| Pub/Sub | 🚧 Planned | ✅ |
| Transactions | 🚧 Planned | ✅ |

## 🧪 Testing

### Run Unit Tests

```bash
cargo test --lib
```

### Run Integration Tests

Requires a running Redis server:

```bash
# Standalone Redis
docker run -d -p 6379:6379 redis:latest

# Run tests
cargo test --test integration_tests
```

### Run Benchmarks

```bash
cargo bench
```

## 📊 Performance

- **Multiplexed mode**: Suitable for high throughput with low overhead
- **Pool mode**: Suitable for low latency with many concurrent operations
- **Zero-copy parsing**: Uses `bytes::Bytes` to avoid unnecessary allocations
- **Efficient slot calculation**: Optimized CRC16 implementation

## 🛠️ Development

### Build Project

```bash
cargo build --release
```

### Format Code

```bash
cargo fmt
```

### Lint with Clippy

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

### Generate Documentation

```bash
# Generate and open documentation
cargo doc --no-deps --open

# Generate documentation with private items (for development)
cargo doc --no-deps --document-private-items --open
```

## 📝 Examples

See more examples in the [`examples/`](examples/) directory:

- [`basic_usage.rs`](examples/basic_usage.rs) - Basic operations
- [`cluster_usage.rs`](examples/cluster_usage.rs) - Redis Cluster with hash tags
- [`pool_strategies.rs`](examples/pool_strategies.rs) - Comparing Multiplexed vs Pool

Run examples:

```bash
cargo run --example basic_usage
cargo run --example cluster_usage
cargo run --example pool_strategies
```

## 📚 API Documentation

Comprehensive API documentation is available at [docs.rs/redis-oxide](https://docs.rs/redis-oxide).

Key modules:

- [`Client`](https://docs.rs/redis-oxide/latest/redis_oxide/struct.Client.html) - High-level Redis client
- [`ConnectionConfig`](https://docs.rs/redis-oxide/latest/redis_oxide/struct.ConnectionConfig.html) - Connection configuration
- [`RedisError`](https://docs.rs/redis-oxide/latest/redis_oxide/enum.RedisError.html) - Error types with MOVED/ASK handling
- [`commands`](https://docs.rs/redis-oxide/latest/redis_oxide/commands/index.html) - Type-safe command builders

## 🗺️ Roadmap

- [x] Core RESP2 protocol
- [x] Standalone Redis support
- [x] Redis Cluster support
- [x] MOVED/ASK redirect handling
- [x] Multiplexed connection
- [x] Connection pooling
- [x] Basic string commands
- [ ] Pipeline support
- [ ] Pub/Sub support
- [ ] Transactions (MULTI/EXEC)
- [ ] Hash, List, Set, Sorted Set commands
- [ ] Lua scripting support
- [ ] Redis Streams
- [ ] RESP3 protocol
- [ ] Sentinel support

## 📄 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## 🤝 Contributing

Contributions are welcome! Please see:

- [CONTRIBUTING.md](CONTRIBUTING.md) - Development and contribution guidelines
- [DOCS.md](DOCS.md) - Documentation writing and maintenance guide

## 📮 Contact

- Repository: https://github.com/yourusername/redis-oxide
- Issues: https://github.com/yourusername/redis-oxide/issues

## 🙏 Acknowledgments

- Inspired by [StackExchange.Redis](https://github.com/StackExchange/StackExchange.Redis)
- Built with [Tokio](https://tokio.rs/)
- Redis protocol reference: https://redis.io/topics/protocol
