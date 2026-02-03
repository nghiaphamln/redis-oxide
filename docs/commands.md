# Commands Overview

redis-oxide provides type-safe command builders for all major Redis data types.

## String Commands

```rust
// Basic operations
client.set("key", "value").await?;
let value: Option<String> = client.get("key").await?;

client.set_ex("key", "value", 60).await?;  // With TTL
client.del("key").await?;
client.exists("key").await?;

// Increment/decrement
client.incr("counter", 1).await?;
client.incrby("counter", 10).await?;
client.decr("counter", 1).await?;
client.decrby("counter", 10).await?;

// Conditional operations
client.set_nx("key", "value").await?;  // Set if not exists
client.set_xx("key", "value").await?;  // Set if exists
```

## Hash Commands

```rust
use redis_oxide::commands::HashCommands;

// Set hash field
client.hset("myhash", "field1", "value1").await?;
client.hset("myhash", "field2", "value2").await?;

// Get hash field
let value: Option<String> = client.hget("myhash", "field1").await?;

// Get all fields
let all: HashMap<String, String> = client.hgetall("myhash").await?;

// Multiple fields
client.hmset("myhash", vec![
    ("field1", "value1"),
    ("field2", "value2"),
]).await?;

let values: Vec<Option<String>> = client.hmget("myhash", vec!["field1", "field2"]).await?;

// Hash metadata
let len: i64 = client.hlen("myhash").await?;
let exists: bool = client.hexists("myhash", "field1").await?;

// Delete field
let deleted: i64 = client.hdel("myhash", "field1").await?;
```

## List Commands

```rust
use redis_oxide::commands::ListCommands;

// Push operations
client.lpush("mylist", "item1").await?;
client.rpush("mylist", "item2").await?;

// Pop operations
let item: Option<String> = client.lpop("mylist").await?;
let item: Option<String> = client.rpop("mylist").await?;

// List metadata
let len: i64 = client.llen("mylist").await?;

// Range
let items: Vec<String> = client.lrange("mylist", 0, -1).await?;  // All items
let items: Vec<String> = client.lrange("mylist", 0, 9).await?;   // First 10

// By index
let item: Option<String> = client.lindex("mylist", 0).await?;  // First

// Set by index
client.lset("mylist", 0, "new_first").await?;
```

## Set Commands

```rust
use redis_oxide::commands::SetCommands;

// Add members
client.sadd("myset", "member1").await?;
client.sadd("myset", vec!["member2", "member3"]).await?;

// Get members
let members: Vec<String> = client.smembers("myset").await?;

// Check membership
let is_member: bool = client.sismember("myset", "member1").await?;

// Set metadata
let size: i64 = client.scard("myset").await?;

// Random members
let random: String = client.srandmember("myset", 1).await?;
let random: Vec<String> = client.srandmember("myset", 3).await?;

// Remove members
let removed: i64 = client.srem("myset", "member1").await?;

// Pop random member
let popped: Option<String> = client.spop("myset").await?;
```

## Sorted Set Commands

```rust
use redis_oxide::commands::SortedSetCommands;

// Add members with score
client.zadd("myzset", "member1", 1.0).await?;
client.zadd("myzset", vec![
    ("member2", 2.0),
    ("member3", 3.0),
]).await?;

// Get score
let score: Option<f64> = client.zscore("myzset", "member1").await?;

// Get rank (0-based, lowest score first)
let rank: Option<i64> = client.zrank("myzset", "member1").await?;

// Get reverse rank (highest score first)
let rev_rank: Option<i64> = client.zrevrank("myzset", "member1").await?;

// Range by rank
let members: Vec<String> = client.zrange("myzset", 0, 9).await?;  // Top 10
let members: Vec<String> = client.zrange("myzset", 0, -1).await?; // All

// ZSet metadata
let size: i64 = client.zcard("myzset").await?;

// Remove members
let removed: i64 = client.zrem("myzset", "member1").await?;
```

## Key Commands

```rust
// Delete keys
let deleted: i64 = client.del(vec!["key1", "key2"]).await?;

// Check existence
let exists: i64 = client.exists(vec!["key1", "key2"]).await?;

// Set expiration
client.expire("key", 60).await?;
client.pexpire("key", 60000).await?;  // Milliseconds

// Get TTL
let ttl: i64 = client.ttl("key").await?;   // Seconds (-1 if no TTL)
let pttl: i64 = client.pttl("key").await?; // Milliseconds
```
