# Getting Started with redis-oxide

This guide will help you quickly get up and running with redis-oxide.

## Prerequisites

- **Rust 1.82 or later** - [Install Rust](https://rustup.rs/)
- **Redis Server 6.0+** - For local testing

### Install Redis Locally

#### Using Docker (Recommended)

```bash
# Run Redis in a container
docker run -d -p 6379:6379 redis:7-alpine

# Verify it's running
docker ps
```

#### macOS (using Homebrew)

```bash
brew install redis
brew services start redis
```

#### Ubuntu/Debian

```bash
sudo apt-get install redis-server
sudo systemctl start redis-server
```

#### Windows

Download from [redis.io](https://redis.io/download) or use Windows Subsystem for Linux (WSL).

## Project Setup

### 1. Create a New Rust Project

```bash
cargo new my-redis-app
cd my-redis-app
```

### 2. Add redis-oxide to Cargo.toml

```toml
[dependencies]
redis-oxide = "0.2.2"
tokio = { version = "1.0", features = ["full"] }
```

### 3. Write Your First Program

Create `src/main.rs`:

```rust
use redis_oxide::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Redis
    let config = ConnectionConfig::new("redis://localhost:6379");
    let client = Client::connect(config).await?;
    
    // Set a value
    client.set("greeting", "Hello, Redis!").await?;
    
    // Get the value back
    if let Some(value) = client.get("greeting").await? {
        println!("Retrieved: {}", value);
    }
    
    Ok(())
}
```

### 4. Run It

```bash
cargo run
```

You should see: `Retrieved: Hello, Redis!`

## Common Patterns

### Handling Multiple Operations

```rust
use redis_oxide::{Client, ConnectionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ConnectionConfig::new("redis://localhost:6379");
    let client = Client::connect(config).await?;
    
    // Set multiple values
    client.set("name", "Alice").await?;
    client.set("age", "30").await?;
    client.set("city", "San Francisco").await?;
    
    // Get multiple values
    let name: String = client.get("name").await?.unwrap_or_default();
    let age: String = client.get("age").await?.unwrap_or_default();
    let city: String = client.get("city").await?.unwrap_or_default();
    
    println!("{} is {} years old and lives in {}", name, age, city);
    
    // Delete values
    client.del("age").await?;
    
    Ok(())
}
```

### Working with Collections

#### Hashes

```rust
use std::collections::HashMap;

let mut fields = HashMap::new();
fields.insert("first_name", "John");
fields.insert("last_name", "Doe");
fields.insert("email", "john@example.com");

client.hset("user:1", fields).await?;

let user_data: HashMap<String, String> = client.hget_all("user:1").await?;
println!("User: {:?}", user_data);
```

#### Lists

```rust
// Push items to a list
client.lpush("tasks", vec!["task3", "task2", "task1"]).await?;

// Get items from list
let tasks: Vec<String> = client.lrange("tasks", 0, -1).await?;
println!("Tasks: {:?}", tasks);

// Pop from list
if let Some(task) = client.lpop("tasks").await? {
    println!("Completed: {}", task);
}
```

#### Sets

```rust
// Add items to a set
client.sadd("tags", vec!["rust", "redis", "async"]).await?;

// Check membership
let is_member: bool = client.sismember("tags", "rust").await?;
println!("Has rust tag: {}", is_member);

// Get all members
let tags: Vec<String> = client.smembers("tags").await?;
println!("All tags: {:?}", tags);
```

#### Sorted Sets

```rust
use std::collections::HashMap;

let mut scores = HashMap::new();
scores.insert("player1", 100.0);
scores.insert("player2", 150.0);
scores.insert("player3", 120.0);

client.zadd("leaderboard", scores).await?;

// Get top 3
let top: Vec<String> = client.zrevrange("leaderboard", 0, 2).await?;
println!("Top players: {:?}", top);
```

### Error Handling

```rust
use redis_oxide::RedisError;

match client.get("key").await {
    Ok(Some(value)) => println!("Value: {}", value),
    Ok(None) => println!("Key not found"),
    Err(RedisError::ConnectionError(e)) => {
        eprintln!("Connection error: {}", e);
        // Implement retry logic or fallback
    }
    Err(RedisError::TimeoutError) => {
        eprintln!("Operation timed out");
        // Handle timeout
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

### Using Pipelines for Bulk Operations

```rust
let mut pipeline = client.pipeline();

for i in 0..100 {
    pipeline.set(format!("key:{}", i), format!("value:{}", i));
}

let results = pipeline.execute().await?;
println!("Executed {} commands", results.len());
```

### Transactions

```rust
let mut transaction = client.transaction().await?;

transaction.watch(vec!["counter"]).await?;

// Read current value
transaction.get("counter");

// Increment and set
transaction.incr("counter");

let results = transaction.exec().await?;
println!("Transaction results: {:?}", results);
```

### Pub/Sub

```rust
let mut pubsub = client.get_pubsub().await?;

// Subscribe to a channel
pubsub.subscribe(vec!["notifications"]).await?;

// Listen for messages
while let Some(msg) = pubsub.next_message().await? {
    println!("Channel: {}, Message: {}", msg.channel, msg.payload);
}
```

## Configuration Options

### Basic Configuration

```rust
use redis_oxide::{ConnectionConfig, ConnectionStrategy, PoolConfig};
use std::time::Duration;

let config = ConnectionConfig::new("redis://localhost:6379")
    // Connection strategy
    .with_strategy(ConnectionStrategy::Pool)
    
    // Pool configuration
    .with_pool_config(
        PoolConfig::new()
            .max_size(20)
            .min_idle(5)
    )
    
    // Timeouts
    .with_connect_timeout(Duration::from_secs(5))
    .with_response_timeout(Duration::from_secs(3))
    
    // Authentication
    .with_password("your-password")
    
    // Database selection
    .with_database(0);

let client = Client::connect(config).await?;
```

### Cluster Configuration

```rust
let config = ConnectionConfig::new("redis://localhost:7000")
    .with_cluster_discovery(true)
    .with_read_from_replicas(true)
    .with_reconnect_attempts(3);

let client = Client::connect(config).await?;
```

### Sentinel Configuration

```rust
let config = ConnectionConfig::new_sentinel(
    vec!["sentinel1:26379", "sentinel2:26379"],
    "mymaster"
)
.with_sentinel_auth("sentinel-password")
.with_password("redis-password");

let client = Client::connect(config).await?;
```

## Testing Your Connection

Create a test file to verify everything works:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use redis_oxide::{Client, ConnectionConfig};

    #[tokio::test]
    async fn test_connection() {
        let config = ConnectionConfig::new("redis://localhost:6379");
        let client = Client::connect(config).await;
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_set_get() {
        let config = ConnectionConfig::new("redis://localhost:6379");
        let client = Client::connect(config).await.unwrap();
        
        client.set("test_key", "test_value").await.unwrap();
        let value: Option<String> = client.get("test_key").await.unwrap();
        assert_eq!(value, Some("test_value".to_string()));
    }
}
```

Run tests:

```bash
cargo test
```

## Next Steps

1. **Read the full documentation**: Visit [docs.rs/redis-oxide](https://docs.rs/redis-oxide)
2. **Check examples**: See the [`examples/`](examples/) directory for more use cases
3. **Explore advanced features**: Learn about pipelines, transactions, Lua scripting
4. **Join the community**: Check out [GitHub Discussions](https://github.com/nghiaphamln/redis-oxide/discussions)

## Troubleshooting

### Connection Refused

```
Error: Connection refused (os error 111)
```

**Solution**: Make sure Redis is running on localhost:6379

```bash
# Check if Redis is running
redis-cli ping

# Should return: PONG
```

### Timeout Errors

```
Error: Operation timed out
```

**Solution**: Increase timeout or check Redis responsiveness

```rust
let config = ConnectionConfig::new("redis://localhost:6379")
    .with_response_timeout(Duration::from_secs(5));
```

### Authentication Failed

```
Error: ERR invalid password
```

**Solution**: Verify password in configuration

```rust
let config = ConnectionConfig::new("redis://localhost:6379")
    .with_password("correct-password");
```

## Getting Help

- **Documentation**: <https://docs.rs/redis-oxide>
- **Issues**: <https://github.com/nghiaphamln/redis-oxide/issues>
- **Discussions**: <https://github.com/nghiaphamln/redis-oxide/discussions>
- **Examples**: Check the [`examples/`](examples/) directory
