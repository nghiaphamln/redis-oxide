# redis-oxide

[![Crates.io](https://img.shields.io/crates/v/redis-oxide.svg)](https://crates.io/crates/redis-oxide)
[![Documentation](https://docs.rs/redis-oxide/badge.svg)](https://docs.rs/redis-oxide)
[![License](https://img.shields.io/crates/l/redis-oxide.svg)](LICENSE)
[![Build Status](https://github.com/yourusername/redis-oxide/workflows/CI/badge.svg)](https://github.com/yourusername/redis-oxide/actions)

**High-performance async Redis client for Rust with automatic cluster support**

`redis-oxide` is a Redis client library similar to StackExchange.Redis for .NET. It automatically detects whether you're connecting to a standalone Redis server or a Redis Cluster, and handles MOVED/ASK redirects transparently.

## 🚀 Features

- **🔍 Automatic topology detection**: Auto-recognizes Standalone Redis or Redis Cluster
- **🔄 MOVED/ASK redirect handling**: Automatically handles slot migrations in cluster mode
- **🏊 Flexible connection strategies**: Supports both Multiplexed connections and Connection Pools
- **🛡️ Type-safe command builders**: Safe API with builder pattern
- **⚡ Async/await**: Fully asynchronous with Tokio runtime
- **🔌 Automatic reconnection**: Reconnects with exponential backoff
- **📊 Comprehensive error handling**: Detailed and clear error types
- **🔧 RESP3 Protocol Support**: Redis 6.0+ protocol with new data types
- **🏗️ Pipeline Support**: Batch multiple commands for better performance
- **💾 Transactions**: MULTI/EXEC/WATCH/DISCARD support
- **📡 Pub/Sub**: Subscribe/Publish messaging
- **🧮 Lua Scripting**: EVAL/EVALSHA with script caching
- **🌊 Redis Streams**: Event sourcing with consumer groups
- **🏛️ Sentinel Support**: High availability with automatic failover
- **✅ High test coverage**: Extensive unit and integration tests

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
redis-oxide = "0.1"
```

## 🚀 Quick Start

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::new("redis://cluster:7000")
        .with_max_redirects(5); // Allow up to 5 redirects

    let client = Client::connect(config).await?;

    // If encountering "MOVED 9916 10.90.6.213:6002",
    // client automatically retries command to 10.90.6.213:6002
    let value = client.get("mykey").await?;
    Ok(())
}
```

## 📚 Comprehensive Examples

### String Operations

```rust
use redis_oxide::{Client, ConnectionConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::new("redis://localhost:6379");
    let client = Client::connect(config).await?;

    // Basic operations
    client.set("key", "value").await?;
    let value: Option<String> = client.get("key").await?;

    // SET with expiration
    client.set_ex("key", "value", Duration::from_secs(60)).await?;

    // SET NX (only if key doesn't exist)
    let set: bool = client.set_nx("key", "value").await?;

    // DELETE multiple keys
    let deleted: i64 = client.del(vec!["key1".to_string(), "key2".to_string()]).await?;

    // EXISTS check
    let exists: i64 = client.exists(vec!["key".to_string()]).await?;

    // EXPIRE and TTL
    client.expire("key", Duration::from_secs(60)).await?;
    let ttl: Option<i64> = client.ttl("key").await?;

    // INCR/DECR operations
    let new_value: i64 = client.incr("counter").await?;
    let new_value: i64 = client.decr("counter").await?;
    let new_value: i64 = client.incr_by("counter", 10).await?;
    let new_value: i64 = client.decr_by("counter", 5).await?;

    Ok(())
}
```

### Hash Operations

```rust
use std::collections::HashMap;

// Hash operations
client.hset("user:1", "name", "Alice").await?;
client.hset("user:1", "age", "30").await?;

let name: Option<String> = client.hget("user:1", "name").await?;
let all_fields: HashMap<String, String> = client.hgetall("user:1").await?;

// Multiple field operations
let fields = vec!["name".to_string(), "age".to_string()];
let values: Vec<Option<String>> = client.hmget("user:1", fields).await?;

let mut field_values = HashMap::new();
field_values.insert("email".to_string(), "alice@example.com".to_string());
field_values.insert("city".to_string(), "New York".to_string());
client.hmset("user:1", field_values).await?;

// Hash utilities
let field_count: i64 = client.hlen("user:1").await?;
let exists: bool = client.hexists("user:1", "email").await?;
let deleted: i64 = client.hdel("user:1", vec!["age".to_string()]).await?;
```

### List Operations

```rust
// List operations
let length: i64 = client.lpush("mylist", vec!["item1".to_string(), "item2".to_string()]).await?;
let length: i64 = client.rpush("mylist", vec!["item3".to_string()]).await?;

let items: Vec<String> = client.lrange("mylist", 0, -1).await?;
let length: i64 = client.llen("mylist").await?;

let first: Option<String> = client.lindex("mylist", 0).await?;
client.lset("mylist", 1, "modified_item").await?;

let popped: Option<String> = client.lpop("mylist").await?;
let popped: Option<String> = client.rpop("mylist").await?;
```

### Set Operations

```rust
use std::collections::HashSet;

// Set operations
let added: i64 = client.sadd("myset", vec!["member1".to_string(), "member2".to_string()]).await?;
let members: HashSet<String> = client.smembers("myset").await?;

let is_member: bool = client.sismember("myset", "member1").await?;
let cardinality: i64 = client.scard("myset").await?;

let random_member: Option<String> = client.srandmember("myset").await?;
let popped: Option<String> = client.spop("myset").await?;
let removed: i64 = client.srem("myset", vec!["member1".to_string()]).await?;
```

### Sorted Set Operations

```rust
use std::collections::HashMap;

// Sorted Set operations
let mut members = HashMap::new();
members.insert("member1".to_string(), 1.0);
members.insert("member2".to_string(), 2.0);
members.insert("member3".to_string(), 3.0);

let added: i64 = client.zadd("myzset", members).await?;
let range: Vec<String> = client.zrange("myzset", 0, -1).await?;

let score: Option<f64> = client.zscore("myzset", "member2").await?;
let rank: Option<i64> = client.zrank("myzset", "member2").await?;
let rev_rank: Option<i64> = client.zrevrank("myzset", "member2").await?;

let cardinality: i64 = client.zcard("myzset").await?;
let removed: i64 = client.zrem("myzset", vec!["member2".to_string()]).await?;
```

### Pipeline Support

```rust
// Pipeline for batch operations
let mut pipeline = client.pipeline();
pipeline.set("key1", "value1");
pipeline.set("key2", "value2");
pipeline.get("key1");
pipeline.incr("counter");

let results = pipeline.execute().await?;
println!("Pipeline results: {:?}", results);
```

### Transactions

```rust
// Atomic transactions
let mut transaction = client.transaction().await?;
transaction.watch(vec!["balance".to_string()]).await?;
transaction.get("balance");
transaction.set("balance", "100");
transaction.incr("transaction_count");

match transaction.exec().await? {
    TransactionResult::Success(results) => {
        println!("Transaction succeeded: {:?}", results);
    }
    TransactionResult::Aborted => {
        println!("Transaction was aborted (watched key changed)");
    }
}
```

### Pub/Sub Messaging

```rust
use futures::StreamExt;

// Publisher
let subscribers = client.publish("news", "Breaking news!").await?;
println!("Message sent to {} subscribers", subscribers);

// Subscriber
let mut subscriber = client.subscriber().await?;
subscriber.subscribe(vec!["news".to_string(), "updates".to_string()]).await?;

while let Some(message) = subscriber.next_message().await? {
    println!("Received: {} on {}", message.payload, message.channel);
}
```

### Lua Scripting

```rust
// Execute Lua script
let script = r#"
    local key = KEYS[1]
    local increment = tonumber(ARGV[1])
    local current = redis.call('GET', key) or 0
    local new_value = tonumber(current) + increment
    redis.call('SET', key, new_value)
    return new_value
"#;

let result: i64 = client.eval(
    script,
    vec!["counter".to_string()],
    vec!["5".to_string()]
).await?;

// Load and execute with EVALSHA
let sha = client.script_load(script).await?;
let result: i64 = client.evalsha(
    &sha,
    vec!["counter".to_string()],
    vec!["3".to_string()]
).await?;

// Script management
let exists = client.script_exists(vec![sha.clone()]).await?;
client.script_flush().await?;
```

### Redis Streams

```rust
use std::collections::HashMap;

// Add entries to stream
let mut fields = HashMap::new();
fields.insert("user_id".to_string(), "123".to_string());
fields.insert("action".to_string(), "login".to_string());

let entry_id = client.xadd("events", "*", fields).await?;

// Read from stream
let streams = vec![("events".to_string(), "0".to_string())];
let messages = client.xread(streams, Some(10), None).await?;

// Consumer groups
client.xgroup_create("events", "processors", "$", true).await?;

let streams = vec![("events".to_string(), ">".to_string())];
let messages = client.xreadgroup(
    "processors",
    "worker-1",
    streams,
    Some(1),
    None
).await?;

// Acknowledge processed messages
for (stream, entries) in messages {
    for entry in entries {
        // Process entry...
        client.xack(&stream, "processors", vec![entry.id]).await?;
    }
}
```

### Redis Sentinel (High Availability)

```rust
use redis_oxide::{Client, ConnectionConfig, SentinelConfig};

// Configure Sentinel
let sentinel_config = SentinelConfig::new("mymaster")
    .add_sentinel("127.0.0.1:26379")
    .add_sentinel("127.0.0.1:26380")
    .add_sentinel("127.0.0.1:26381")
    .with_password("sentinel_password");

let config = ConnectionConfig::new_with_sentinel(sentinel_config);
let client = Client::connect(config).await?;

// Client automatically connects to current master
// and handles failover transparently
client.set("key", "value").await?;
```

### RESP3 Protocol Support

```rust
use redis_oxide::{ConnectionConfig, ProtocolVersion};

// Use RESP3 protocol (Redis 6.0+)
let config = ConnectionConfig::new("redis://localhost:6379")
    .with_protocol_version(ProtocolVersion::Resp3);

let client = Client::connect(config).await?;

// RESP3 provides richer data types like maps, sets, booleans, etc.
// The client automatically handles protocol negotiation
```

## ⚙️ Configuration

### Connection Configuration

```rust
use redis_oxide::{ConnectionConfig, TopologyMode, ProtocolVersion};
use std::time::Duration;

let config = ConnectionConfig::new("redis://localhost:6379")
    .with_password("secret")                    // Password (optional)
    .with_database(0)                           // Database number
    .with_connect_timeout(Duration::from_secs(5))
    .with_operation_timeout(Duration::from_secs(30))
    .with_topology_mode(TopologyMode::Auto)    // Auto, Standalone, or Cluster
    .with_protocol_version(ProtocolVersion::Resp3) // RESP2 or RESP3
    .with_max_redirects(3);                     // Max retries for cluster redirects
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
use redis_oxide::{ConnectionConfig, PoolConfig, PoolStrategy};
use std::time::Duration;

let pool_config = PoolConfig {
    strategy: PoolStrategy::Pool,
    max_size: 20,                           // Max 20 connections
    min_idle: 5,                            // Keep at least 5 idle connections
    connection_timeout: Duration::from_secs(5),
};

let mut config = ConnectionConfig::new("redis://localhost:6379");
config.pool = pool_config;
```

## 🎯 Supported Commands

### String Operations (10 commands)
- `GET`, `SET`, `SET EX`, `SET NX`
- `DEL`, `EXISTS`, `EXPIRE`, `TTL`
- `INCR`, `DECR`, `INCRBY`, `DECRBY`

### Hash Operations (8 commands)
- `HGET`, `HSET`, `HDEL`, `HGETALL`
- `HMGET`, `HMSET`, `HLEN`, `HEXISTS`

### List Operations (8 commands)
- `LPUSH`, `RPUSH`, `LPOP`, `RPOP`
- `LRANGE`, `LLEN`, `LINDEX`, `LSET`

### Set Operations (7 commands)
- `SADD`, `SREM`, `SMEMBERS`, `SISMEMBER`
- `SCARD`, `SPOP`, `SRANDMEMBER`

### Sorted Set Operations (7 commands)
- `ZADD`, `ZREM`, `ZRANGE`, `ZSCORE`
- `ZCARD`, `ZRANK`, `ZREVRANK`

### Pub/Sub Operations (5 commands)
- `PUBLISH`, `SUBSCRIBE`, `UNSUBSCRIBE`
- `PSUBSCRIBE`, `PUNSUBSCRIBE`

### Lua Scripting (5 commands)
- `EVAL`, `EVALSHA`, `SCRIPT LOAD`
- `SCRIPT EXISTS`, `SCRIPT FLUSH`

### Redis Streams (7 commands)
- `XADD`, `XREAD`, `XRANGE`, `XLEN`
- `XGROUP CREATE`, `XREADGROUP`, `XACK`

### Transactions (4 commands)
- `MULTI`, `EXEC`, `WATCH`, `DISCARD`

### **Total: 60+ Redis commands supported**

## 🏗️ Architecture

### Automatic Topology Detection

```rust
// The client automatically detects the topology
let client = Client::connect(config).await?;

match client.topology_type() {
    TopologyType::Standalone => println!("Connected to standalone Redis"),
    TopologyType::Cluster => println!("Connected to Redis Cluster"),
}
```

### Error Handling

```rust
use redis_oxide::{RedisError, RedisResult};

match client.get("key").await {
    Ok(Some(value)) => println!("Value: {}", value),
    Ok(None) => println!("Key not found"),
    Err(RedisError::Connection(msg)) => println!("Connection error: {}", msg),
    Err(RedisError::Cluster(msg)) => println!("Cluster error: {}", msg),
    Err(RedisError::Protocol(msg)) => println!("Protocol error: {}", msg),
    Err(e) => println!("Other error: {}", e),
}
```

## 🧪 Testing

Run the test suite:

```bash
# Unit tests
cargo test --lib

# Integration tests (requires Redis server)
cargo test --test test_simple_integration

# All tests
cargo test

# With coverage
cargo tarpaulin --out Html
```

## 📊 Performance

The library is optimized for performance with:

- **Zero-copy operations** where possible
- **Connection pooling** and **multiplexing**
- **Pipeline support** for batch operations
- **Async/await** throughout
- **Efficient RESP protocol** implementation
- **LTO and optimization** in release builds

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

### Development Setup

```bash
git clone https://github.com/yourusername/redis-oxide.git
cd redis-oxide
cargo build
cargo test
```

## 📄 License

This project is licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## 🙏 Acknowledgments

- Inspired by [StackExchange.Redis](https://github.com/StackExchange/StackExchange.Redis) for .NET
- Built with [Tokio](https://tokio.rs/) for async runtime
- Uses [testcontainers](https://github.com/testcontainers/testcontainers-rs) for integration testing

---

**redis-oxide** - High-performance async Redis client for Rust 🦀