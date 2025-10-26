# redis-oxide

[![Crates.io](https://img.shields.io/crates/v/redis-oxide.svg)](https://crates.io/crates/redis-oxide)
[![Docs.rs](https://docs.rs/redis-oxide/badge.svg)](https://docs.rs/redis-oxide)
[![License: MIT](https://img.shields.io/crates/l/redis-oxide.svg)](https://github.com/nghiaphamln/redis-oxide#license)
[![CI](https://github.com/nghiaphamln/redis-oxide/workflows/CI/badge.svg)](https://github.com/nghiaphamln/redis-oxide/actions)

A high-performance, async Redis client for Rust with automatic cluster detection, comprehensive Redis feature support, and flexible connection strategies. Inspired by StackExchange.Redis for .NET.

## ✨ Features

- 🚀 **High Performance**: Optimized protocol encoding/decoding with memory pooling
- 🔄 **Automatic Topology Detection**: Auto-detects Standalone Redis or Redis Cluster
- 🎯 **Comprehensive Command Support**: Full Redis command coverage including latest features
- 🔗 **Flexible Connection Strategies**: Multiplexed connections and traditional connection pools
- 📡 **Advanced Features**: Pub/Sub, Streams, Lua scripting, Transactions, Pipelines
- 🛡️ **High Availability**: Redis Sentinel support with automatic failover
- 🔧 **Protocol Support**: Both RESP2 and RESP3 protocols
- ⚡ **Async/Await**: Fully asynchronous with Tokio runtime
- 🔄 **Automatic Reconnection**: Smart reconnection with exponential backoff
- 🎨 **Type-Safe APIs**: Builder patterns and compile-time safety
- 📊 **Cross-Platform**: Linux, macOS, and Windows support

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
redis-oxide = "0.2.2"
tokio = { version = "1.0", features = ["full"] }
```

## 🚀 Quick Start

### Basic Usage

```rust
use redis_oxide::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Redis (automatically detects topology)
    let config = ConnectionConfig::new("redis://localhost:6379");
    let client = Client::connect(config).await?;
    
    // Basic operations
    client.set("key", "Hello, Redis!").await?;
    if let Some(value) = client.get("key").await? {
        println!("Value: {}", value);
    }
    
    Ok(())
}
```

### Redis Cluster

```rust
use redis_oxide::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::new("redis://localhost:7000")
        .with_cluster_discovery(true);
    
    let client = Client::connect(config).await?;
    
    // Automatic slot mapping and MOVED/ASK redirect handling
    client.set("cluster_key", "value").await?;
    
    Ok(())
}
```

## 🎯 Core Operations

### String Operations

```rust
// SET/GET with various options
client.set("key", "value").await?;
client.set_ex("key", "value", Duration::from_secs(60)).await?;
client.set_nx("key", "value").await?; // Only if not exists
client.set_xx("key", "value").await?; // Only if exists

let value: Option<String> = client.get("key").await?;
let values: Vec<Option<String>> = client.mget(vec!["key1", "key2"]).await?;

// Atomic operations
let new_value: i64 = client.incr("counter").await?;
let new_value: i64 = client.incr_by("counter", 10).await?;
```

### Hash Operations

```rust
use std::collections::HashMap;

// Hash operations
client.hset("hash", "field", "value").await?;
let mut fields = HashMap::new();
fields.insert("field1", "value1");
fields.insert("field2", "value2");
client.hset_multiple("hash", fields).await?;

let value: Option<String> = client.hget("hash", "field").await?;
let all: HashMap<String, String> = client.hget_all("hash").await?;
```

### List Operations

```rust
// List operations
client.lpush("list", vec!["item1", "item2"]).await?;
client.rpush("list", vec!["item3", "item4"]).await?;

let items: Vec<String> = client.lrange("list", 0, -1).await?;
let item: Option<String> = client.lpop("list").await?;
let length: i64 = client.llen("list").await?;
```

### Set Operations

```rust
// Set operations
client.sadd("set", vec!["member1", "member2"]).await?;
let members: Vec<String> = client.smembers("set").await?;
let is_member: bool = client.sismember("set", "member1").await?;
let removed: i64 = client.srem("set", vec!["member1"]).await?;
```

### Sorted Set Operations

```rust
use std::collections::HashMap;

// Sorted set operations
let mut scores = HashMap::new();
scores.insert("member1", 100.0);
scores.insert("member2", 200.0);
client.zadd("zset", scores).await?;

let members: Vec<String> = client.zrange("zset", 0, -1).await?;
let score: Option<f64> = client.zscore("zset", "member1").await?;
let rank: Option<i64> = client.zrank("zset", "member1").await?;
```

## 🔧 Advanced Features

### Pipeline Operations

Batch multiple commands for improved performance:

```rust
let mut pipeline = client.pipeline();
pipeline.set("key1", "value1");
pipeline.set("key2", "value2");
pipeline.get("key1");
pipeline.incr("counter");

let results = pipeline.execute().await?;
```

### Transactions

ACID transactions with optimistic locking:

```rust
let mut transaction = client.transaction().await?;
transaction.watch(vec!["balance"]).await?;

transaction.get("balance");
transaction.set("balance", "new_value");
transaction.set("last_updated", "timestamp");

let results = transaction.exec().await?;
```

### Lua Scripting

Execute Lua scripts with automatic EVALSHA caching:

```rust
use redis_oxide::{Script, ScriptManager};

let script = Script::new(r#"
    local current = redis.call('GET', KEYS[1]) or 0
    local increment = tonumber(ARGV[1])
    local new_value = tonumber(current) + increment
    redis.call('SET', KEYS[1], new_value)
    return new_value
"#);

let result: i64 = script.execute(
    &client,
    vec!["counter"],
    vec!["5"]
).await?;
```

### Pub/Sub Messaging

Real-time messaging with pattern matching:

```rust
use redis_oxide::PubSub;

let mut pubsub = client.get_pubsub().await?;

// Subscribe to channels
pubsub.subscribe(vec!["channel1", "channel2"]).await?;
pubsub.psubscribe(vec!["news.*", "updates.*"]).await?;

// Listen for messages
while let Some(msg) = pubsub.next_message().await? {
    println!("Channel: {}, Message: {}", msg.channel, msg.payload);
}
```

### Redis Streams

Event sourcing and stream processing:

```rust
use redis_oxide::streams::{StreamEntry, StreamReadOptions};

// Add entries to stream
client.xadd("mystream", "*", vec![("field1", "value1"), ("field2", "value2")]).await?;

// Read from stream
let options = StreamReadOptions::new().count(10).block(1000);
let entries: Vec<StreamEntry> = client.xread(vec![("mystream", "0")], options).await?;

// Consumer groups
client.xgroup_create("mystream", "mygroup", "0", false).await?;
let entries = client.xreadgroup("mygroup", "consumer1", vec![("mystream", ">")], options).await?;
```

## 🏗️ Architecture

### Connection Strategies

#### Multiplexed Connections (Default)
Shares a single connection across multiple tasks for optimal resource usage:

```rust
let config = ConnectionConfig::new("redis://localhost:6379")
    .with_strategy(ConnectionStrategy::Multiplexed);
```

#### Connection Pool
Traditional connection pooling for high-throughput scenarios:

```rust
let config = ConnectionConfig::new("redis://localhost:6379")
    .with_strategy(ConnectionStrategy::Pool)
    .with_pool_config(PoolConfig::new().max_size(20));
```

### Cluster Support

Automatic cluster topology discovery and slot management:

```rust
let config = ConnectionConfig::new("redis://cluster-node1:7000")
    .with_cluster_discovery(true)
    .with_read_from_replicas(true);

let client = Client::connect(config).await?;
// Automatically handles MOVED/ASK redirects
```

### High Availability with Sentinel

```rust
let config = ConnectionConfig::new_sentinel(
    vec!["sentinel1:26379", "sentinel2:26379"],
    "mymaster"
).with_sentinel_auth("password");

let client = Client::connect(config).await?;
// Automatic failover handling
```

## 🛠️ Development

### Prerequisites

- Rust 1.82 or later
- Redis server 6.0+ for testing

### Setup

```bash
# Clone the repository
git clone https://github.com/nghiaphamln/redis-oxide.git
cd redis-oxide

# Build the project
cargo build

# Run tests (requires Redis server)
docker run --rm -d -p 6379:6379 redis:7-alpine
cargo test

# Run examples
cargo run --example basic_usage
cargo run --example cluster_usage
```

### Contributing

We welcome contributions! Please:

1. **Fork and clone** the repository
2. **Create a feature branch** from `main`
3. **Follow code style**: Run `cargo fmt` and `cargo clippy`
4. **Add tests** for new features
5. **Update documentation** as needed
6. **Submit a pull request** with a clear description

### Code Quality

Before submitting a PR, ensure:

```bash
# Format code
cargo fmt --all

# Run clippy
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run tests
cargo test --all-features

# Build release
cargo build --release
```

## 🎯 Roadmap

### Current Version (v0.2.2)
- ✅ Basic Redis operations support
- ✅ Cluster mode with automatic topology detection
- ✅ Connection pooling strategies (Multiplexed & Connection Pool)
- ✅ Pub/Sub messaging and Redis Streams
- ✅ Lua scripting with EVALSHA caching
- ✅ Sentinel support for high availability
- ✅ Transactions and Pipelines

### Upcoming Features

#### v0.3.0 - Performance & Memory Optimizations
- [ ] Zero-copy parsing for RESP3
- [ ] Memory pooling and buffer recycling
- [ ] Adaptive connection pool sizing
- [ ] Comprehensive benchmarking suite

#### v0.4.0 - Monitoring & Observability
- [ ] OpenTelemetry integration
- [ ] Built-in metrics and health checks
- [ ] Distributed tracing support
- [ ] Performance monitoring dashboard

## ⚡ Performance

Redis Oxide is designed for high performance:

- **Zero-copy parsing** where possible
- **Connection multiplexing** reduces overhead
- **Automatic pipelining** for bulk operations
- **RESP3 protocol** support for reduced bandwidth
- **Memory pooling** to minimize allocations

## 🔧 Configuration

### Connection Configuration

```rust
use redis_oxide::{ConnectionConfig, ConnectionStrategy, PoolConfig};

let config = ConnectionConfig::new("redis://localhost:6379")
    .with_strategy(ConnectionStrategy::Pool)
    .with_pool_config(
        PoolConfig::new()
            .max_size(20)
            .min_idle(5)
            .connection_timeout(Duration::from_secs(5))
    )
    .with_connect_timeout(Duration::from_secs(3))
    .with_response_timeout(Duration::from_secs(2))
    .with_reconnect_attempts(3)
    .with_password("your-password")
    .with_database(0);
```

### SSL/TLS Support

```rust
let config = ConnectionConfig::new("rediss://localhost:6380")
    .with_tls_config(
        TlsConfig::new()
            .ca_cert_path("/path/to/ca.crt")
            .client_cert_path("/path/to/client.crt")
            .client_key_path("/path/to/client.key")
    );
```

## 🐛 Error Handling

Redis Oxide provides comprehensive error types:

```rust
use redis_oxide::RedisError;

match client.get("key").await {
    Ok(value) => println!("Value: {:?}", value),
    Err(RedisError::ConnectionError(e)) => eprintln!("Connection failed: {}", e),
    Err(RedisError::TimeoutError) => eprintln!("Operation timed out"),
    Err(RedisError::ClusterError(e)) => eprintln!("Cluster error: {}", e),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## 🔍 Examples

Check out the [`examples/`](examples/) directory for complete examples:

- [`basic_usage.rs`](examples/basic_usage.rs) - Basic Redis operations
- [`cluster_usage.rs`](examples/cluster_usage.rs) - Redis Cluster features
- [`pool_strategies.rs`](examples/pool_strategies.rs) - Connection strategies

## 📖 Documentation

- **API Documentation**: <https://docs.rs/redis-oxide>
- **Examples**: See [`examples/`](examples/) directory
- **Contributing Guide**: See above development section

## ⚖️ Compatibility

### Rust Version Support
- **MSRV**: Rust 1.82.0
- **Edition**: 2021 (stable)
- **Platforms**: Linux, macOS, Windows

### Redis Version Support
- **Redis 6.0+**: Full support
- **Redis 7.0+**: Recommended for optimal performance
- **Redis Cluster**: Supported
- **Redis Sentinel**: Supported

### Known Issues

There is a temporary ecosystem issue with the `home` crate and `edition2024` feature flag. This affects transitive dependencies but not the library functionality. Use Rust 1.82.0+ to avoid this issue.

## 📄 License

This project is licensed under either of the following, at your option:

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

## 🤝 Support

- **Documentation**: <https://docs.rs/redis-oxide>
- **Issues**: <https://github.com/nghiaphamln/redis-oxide/issues>
- **Discussions**: <https://github.com/nghiaphamln/redis-oxide/discussions>

---

Made with ❤️ by the Redis Oxide team