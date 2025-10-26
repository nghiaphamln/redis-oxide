# Troubleshooting Guide

This guide covers common issues and their solutions when using redis-oxide.

## Connection Issues

### Error: "Connection refused"

**Error Message:**
```
Error: Connection refused (os error 111)
```

**Causes:**
- Redis server is not running
- Redis is running on a different host/port
- Firewall blocking connection

**Solutions:**

1. **Check if Redis is running:**
   ```bash
   redis-cli ping
   # Should return: PONG
   ```

2. **Start Redis:**
   ```bash
   # Using Docker
   docker run -d -p 6379:6379 redis:7-alpine
   
   # Using Homebrew (macOS)
   brew services start redis
   
   # Using systemctl (Linux)
   sudo systemctl start redis-server
   ```

3. **Verify connection URL:**
   ```rust
   // Make sure the URL is correct
   let config = ConnectionConfig::new("redis://localhost:6379");
   //                                     ^^^^^^^^ host
   //                                               ^^^^ port
   ```

### Error: "Address already in use"

**Causes:**
- Another instance of Redis is already running on the same port
- The port wasn't properly released from a previous run

**Solutions:**

```bash
# Find process using port 6379
lsof -i :6379  # macOS/Linux
netstat -ano | findstr :6379  # Windows

# Kill the process (replace PID with actual process ID)
kill -9 <PID>  # macOS/Linux
taskkill /PID <PID> /F  # Windows

# Or use a different port
docker run -d -p 6380:6379 redis:7-alpine
```

### Error: "Connection timed out"

**Error Message:**
```
Error: Operation timed out
```

**Causes:**
- Redis is unresponsive
- Network latency is high
- Timeout value is too low
- Redis is overloaded

**Solutions:**

1. **Increase timeout:**
   ```rust
   use std::time::Duration;
   
   let config = ConnectionConfig::new("redis://localhost:6379")
       .with_connect_timeout(Duration::from_secs(10))
       .with_response_timeout(Duration::from_secs(5));
   ```

2. **Check Redis responsiveness:**
   ```bash
   redis-cli --latency
   redis-cli --stat
   ```

3. **Monitor Redis memory:**
   ```bash
   redis-cli INFO memory
   ```

### Error: "Too many open files"

**Causes:**
- Too many simultaneous connections
- File descriptor limit reached

**Solutions:**

1. **Increase file descriptor limit (Linux):**
   ```bash
   # Check current limit
   ulimit -n
   
   # Increase limit
   ulimit -n 65536
   ```

2. **Use connection pooling:**
   ```rust
   use redis_oxide::{ConnectionConfig, ConnectionStrategy, PoolConfig};
   
   let config = ConnectionConfig::new("redis://localhost:6379")
       .with_strategy(ConnectionStrategy::Pool)
       .with_pool_config(
           PoolConfig::new()
               .max_size(50)
               .min_idle(10)
       );
   ```

## Authentication Issues

### Error: "ERR invalid password"

**Error Message:**
```
Error: ERR invalid password
```

**Causes:**
- Wrong password provided
- Redis requires authentication but no password given
- Password contains special characters that need escaping

**Solutions:**

1. **Verify password:**
   ```bash
   redis-cli -a "your-password" ping
   ```

2. **Set password in configuration:**
   ```rust
   let config = ConnectionConfig::new("redis://localhost:6379")
       .with_password("your-password");
   ```

3. **Include password in URL:**
   ```rust
   let config = ConnectionConfig::new("redis://:your-password@localhost:6379");
   ```

### Error: "ERR Client sent AUTH, but no password is set"

**Causes:**
- Providing password when Redis doesn't require it

**Solution:**
```rust
// Remove the password from configuration
let config = ConnectionConfig::new("redis://localhost:6379");
// Don't call .with_password()
```

## Cluster Issues

### Error: "MOVED" redirects

**Error Message:**
```
Error: MOVED <slot> <host:port>
```

**Causes:**
- Cluster topology changed
- Slot is on a different node
- Cluster discovery is not enabled

**Solutions:**

1. **Enable cluster discovery:**
   ```rust
   let config = ConnectionConfig::new("redis://localhost:7000")
       .with_cluster_discovery(true);
   ```

2. **Verify cluster is running:**
   ```bash
   redis-cli -c cluster info
   redis-cli -c cluster nodes
   ```

### Error: "CLUSTERDOWN Hash slot not served"

**Causes:**
- Cluster is down
- No nodes available for the slot
- Network partition

**Solutions:**

1. **Check cluster status:**
   ```bash
   redis-cli -c cluster info
   ```

2. **Verify all nodes are running:**
   ```bash
   redis-cli -c cluster nodes
   ```

3. **Wait for cluster to recover:**
   - If this happened after a failure, wait for automatic recovery
   - Check cluster logs

## Pipeline Issues

### Unexpected results from pipeline

**Causes:**
- Commands executed in wrong order
- Result mapping is incorrect
- Pipeline not properly executed

**Solutions:**

```rust
// Make sure to call execute()
let mut pipeline = client.pipeline();
pipeline.set("key1", "value1");
pipeline.get("key1");

// Execute the pipeline
let results = pipeline.execute().await?;

// Results are in order: [OK, Some("value1")]
```

## Transaction Issues

### WATCH key changed

**Error Message:**
```
Error: Transaction aborted (key modified)
```

**Causes:**
- Another client modified a watched key
- Transaction logic is not idempotent

**Solutions:**

1. **Retry the transaction:**
   ```rust
   let max_retries = 3;
   let mut attempt = 0;
   
   loop {
       let mut tx = client.transaction().await?;
       tx.watch(vec!["balance"]).await?;
       
       match tx.exec().await {
           Ok(results) => break,
           Err(_) if attempt < max_retries => {
               attempt += 1;
               continue;
           }
           Err(e) => return Err(e),
       }
   }
   ```

2. **Use optimistic locking with CAS:**
   ```rust
   let mut tx = client.transaction().await?;
   tx.watch(vec!["counter"]).await?;
   
   let current: i64 = tx.get("counter").await?.unwrap_or(0);
   tx.set("counter", current + 1);
   
   tx.exec().await?;
   ```

## Memory Issues

### Redis memory limit exceeded

**Error Message:**
```
Error: OOM command not allowed when used memory > 'maxmemory'
```

**Causes:**
- Too much data stored in Redis
- No eviction policy set
- Memory leaks in your application

**Solutions:**

1. **Set eviction policy:**
   ```bash
   redis-cli CONFIG SET maxmemory-policy allkeys-lru
   ```

2. **Check memory usage:**
   ```bash
   redis-cli INFO memory
   redis-cli DBSIZE
   ```

3. **Clear unnecessary data:**
   ```bash
   redis-cli FLUSHDB  # Clear current database
   redis-cli FLUSHALL # Clear all databases (careful!)
   ```

### Memory leak in application

**Symptoms:**
- Memory usage keeps growing
- Redis memory fills up over time

**Solutions:**

1. **Set expiration on keys:**
   ```rust
   use std::time::Duration;
   
   client.set_ex("temp_key", "value", Duration::from_secs(3600)).await?;
   ```

2. **Monitor key expiration:**
   ```bash
   redis-cli MONITOR
   ```

3. **Clear old data periodically:**
   ```rust
   // Use SCAN and delete old keys
   let keys: Vec<String> = client.scan(0).await?;
   for key in keys {
       let ttl: i64 = client.ttl(&key).await?;
       if ttl == -1 {
           // No expiration, consider deleting
           client.del(&key).await?;
       }
   }
   ```

## Performance Issues

### Slow operations

**Symptoms:**
- High latency on commands
- Timeouts occurring
- Application hanging

**Solutions:**

1. **Profile with SLOWLOG:**
   ```bash
   redis-cli SLOWLOG GET 10
   redis-cli SLOWLOG LEN
   redis-cli SLOWLOG RESET
   ```

2. **Use pipelining:**
   ```rust
   let mut pipeline = client.pipeline();
   for i in 0..1000 {
       pipeline.set(format!("key{}", i), i);
   }
   pipeline.execute().await?;
   ```

3. **Monitor connections:**
   ```bash
   redis-cli CLIENT LIST
   redis-cli INFO stats
   ```

### High CPU usage

**Causes:**
- Inefficient data structures
- Inefficient Lua scripts
- KEYS command on large databases

**Solutions:**

1. **Avoid KEYS command:**
   ```rust
   // ❌ Bad - scans all keys
   let keys: Vec<String> = client.keys("pattern*").await?;
   
   // ✅ Good - incremental scanning
   let mut cursor = 0;
   loop {
       let (new_cursor, keys) = client.scan(cursor).await?;
       cursor = new_cursor;
       if cursor == 0 { break; }
   }
   ```

2. **Optimize Lua scripts:**
   ```rust
   // Minimize operations in Lua
   let script = Script::new(r#"
       local value = redis.call('GET', KEYS[1])
       if value then
           return tonumber(value) + 1
       else
           return 1
       end
   "#);
   ```

## Pub/Sub Issues

### Messages not being received

**Causes:**
- Subscriber not subscribed yet
- Publisher publishing before subscriber ready
- Message already published

**Solutions:**

```rust
// Ensure subscriber is ready before publisher sends
let mut pubsub = client.get_pubsub().await?;
pubsub.subscribe(vec!["notifications"]).await?;

// Give it a moment to subscribe
tokio::time::sleep(Duration::from_millis(100)).await;

// Now publish
client.publish("notifications", "Hello").await?;
```

### Pattern subscription not working

**Causes:**
- Incorrect pattern syntax
- Pattern not matching published channels

**Solutions:**

```rust
let mut pubsub = client.get_pubsub().await?;

// Use PSUBSCRIBE for patterns
pubsub.psubscribe(vec!["notifications.*", "alerts.*"]).await?;

while let Some(msg) = pubsub.next_message().await? {
    println!("Pattern: {}, Channel: {}, Message: {}", 
             msg.pattern, msg.channel, msg.payload);
}
```

## Sentinel Issues

### Error: "Sentinel is down"

**Causes:**
- Sentinel server is not running
- Sentinel configuration is incorrect
- Network connectivity issue

**Solutions:**

1. **Verify Sentinel is running:**
   ```bash
   redis-cli -p 26379 PING
   ```

2. **Check Sentinel configuration:**
   ```bash
   redis-cli -p 26379 INFO sentinel
   ```

3. **Verify master/replica status:**
   ```bash
   redis-cli -p 26379 SENTINEL masters
   redis-cli -p 26379 SENTINEL slaves mymaster
   ```

## Debugging

### Enable detailed logging

```rust
use tracing_subscriber;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    
    // Your code here
}
```

### Monitor Redis commands

```bash
# In a separate terminal, watch all commands
redis-cli MONITOR
```

### Analyze performance

```bash
# Check statistics
redis-cli INFO stats
redis-cli INFO clients
redis-cli INFO memory

# Monitor in real-time
redis-cli --stat
```

## Getting Help

If you can't find a solution:

1. **Check the documentation**: <https://docs.rs/redis-oxide>
2. **Search issues**: <https://github.com/nghiaphamln/redis-oxide/issues>
3. **Open a new issue**: Include error message, code sample, and environment
4. **Discussions**: <https://github.com/nghiaphamln/redis-oxide/discussions>
