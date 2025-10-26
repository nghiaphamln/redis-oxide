//! High-performance async Redis client for Rust
//!
//! `redis-oxide` is a comprehensive Redis client library for Rust.
//! It automatically detects whether you're connecting to a standalone Redis server
//! or a Redis Cluster, and handles MOVED/ASK redirects transparently.
//!
//! # 🚀 Features
//!
//! - **Automatic topology detection**: Auto-recognizes Standalone Redis or Redis Cluster
//! - **MOVED/ASK redirect handling**: Automatically handles slot migrations in cluster mode
//! - **Flexible connection strategies**: Supports both Multiplexed connections and Connection Pools
//! - **Type-safe command builders**: Safe API with builder pattern
//! - **Async/await**: Fully asynchronous with Tokio runtime
//! - **Automatic reconnection**: Reconnects with exponential backoff
//! - **Comprehensive error handling**: Detailed and clear error types
//! - **High test coverage**: Extensive unit and integration tests
//! - **Cross-platform support**: Works on Linux, macOS, and Windows
//! - **Pub/Sub Support**: Built-in publish/subscribe messaging
//! - **Streams Support**: Full Redis Streams functionality for event sourcing
//! - **Lua Scripting**: Execute Lua scripts with automatic EVALSHA caching
//! - **Sentinel Support**: High availability with Redis Sentinel
//! - **Transaction Support**: MULTI/EXEC transaction handling
//! - **Pipeline Support**: Batch multiple commands for improved performance
//! - **RESP2/RESP3 Protocol Support**: Full support for both protocol versions
//! - **Connection Pooling**: Configurable connection pooling strategies
//! - **Multiplexed Connections**: Single connection shared across tasks
//! - **Hash Operations**: Complete set of hash data type operations
//! - **List Operations**: Complete set of list data type operations
//! - **Set Operations**: Complete set of set data type operations
//! - **Sorted Set Operations**: Complete set of sorted set data type operations
//! - **String Operations**: Complete set of string data type operations
//! - **HyperLogLog Operations**: Support for probabilistic data structures
//! - **Geo Operations**: Support for geospatial data types
//! - **Performance Optimizations**: Memory pooling and protocol optimizations
//! - **Monitoring & Metrics**: Built-in observability features
//! - **Configurable Timeouts**: Connect, operation, and redirect timeout controls
//! - **Authentication Support**: Password and access control support
//!
//! # 📦 Installation
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! redis-oxide = "0.2.2"
//! tokio = { version = "1.0", features = ["full"] }
//! ```
//!
//! # 🛠️ Quick Start
//!
//! ## Basic Connection (Standalone)
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create configuration
//!     let config = ConnectionConfig::new("redis://localhost:6379");
//!     
//!     // Connect (automatically detects topology)
//!     let client = Client::connect(config).await?;
//!     
//!     // SET and GET
//!     client.set("mykey", "Hello, Redis!").await?;
//!     if let Some(value) = client.get("mykey").await? {
//!         println!("Value: {}", value);
//!     }
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Redis Cluster Connection
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Provide multiple seed nodes
//!     let config = ConnectionConfig::new(
//!         "redis://node1:7000,node2:7001,node3:7002"
//!     );
//!     
//!     let client = Client::connect(config).await?;
//!     
//!     // Client automatically handles MOVED redirects
//!     client.set("key", "value").await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! # 🎯 Supported Operations
//!
//! ## String Operations
//!
//! ```no_run
//! # use redis_oxide::{Client, ConnectionConfig};
//! # use std::time::Duration;
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let config = ConnectionConfig::new("redis://localhost:6379");
//! # let client = Client::connect(config).await?;
//! // GET
//! let value: Option<String> = client.get("key").await?;
//!
//! // SET
//! client.set("key", "value").await?;
//!
//! // SET with expiration
//! client.set_ex("key", "value", Duration::from_secs(60)).await?;
//!
//! // SET NX (only if key doesn't exist)
//! let set: bool = client.set_nx("key", "value").await?;
//! assert!(set);
//!
//! // DELETE
//! let deleted: i64 = client.del(vec!["key1".to_string(), "key2".to_string()]).await?;
//!
//! // EXISTS
//! let exists: i64 = client.exists(vec!["key".to_string()]).await?;
//!
//! // EXPIRE
//! client.expire("key", Duration::from_secs(60)).await?;
//!
//! // TTL
//! let ttl: Option<i64> = client.ttl("key").await?;
//!
//! // INCR/DECR
//! let new_value: i64 = client.incr("counter").await?;
//! let new_value: i64 = client.decr("counter").await?;
//!
//! // INCRBY/DECRBY
//! let new_value: i64 = client.incr_by("counter", 10).await?;
//! let new_value: i64 = client.decr_by("counter", 5).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Hash Operations
//!
//! ```no_run,ignore
//! # use redis_oxide::{Client, ConnectionConfig};
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let config = ConnectionConfig::new("redis://localhost:6379");
//! # let client = Client::connect(config).await?;
//! // HSET
//! client.hset("myhash", "field1", "value1").await?;
//! client.hset("myhash", "field2", "value2").await?;
//!
//! // HGET
//! if let Some(value) = client.hget("myhash", "field1").await? {
//!     println!("field1: {}", value);
//! }
//!
//! // HGETALL
//! let all_fields = client.hgetall("myhash").await?;
//! println!("All hash fields: {:?}", all_fields);
//!
//! // HMGET (multiple get)
//! let values = client.hmget("myhash", vec!["field1".to_string(), "field2".to_string()]).await?;
//! println!("Multiple fields: {:?}", values);
//!
//! // HMSET (multiple set)
//! use std::collections::HashMap;
//! let mut fields = HashMap::new();
//! fields.insert("field3".to_string(), "value3".to_string());
//! fields.insert("field4".to_string(), "value4".to_string());
//! client.hmset("myhash", fields).await?;
//!
//! // HDEL
//! let deleted: i64 = client.hdel("myhash", vec!["field1".to_string(), "field2".to_string()]).await?;
//!
//! // HEXISTS
//! let exists: bool = client.hexists("myhash", "field1").await?;
//!
//! // HLEN
//! let len: i64 = client.hlen("myhash").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## List Operations
//!
//! ```no_run
//! # use redis_oxide::{Client, ConnectionConfig};
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let config = ConnectionConfig::new("redis://localhost:6379");
//! # let client = Client::connect(config).await?;
//! // LPUSH
//! client.lpush("mylist", vec!["item1".to_string(), "item2".to_string()]).await?;
//!
//! // RPUSH
//! client.rpush("mylist", vec!["item3".to_string(), "item4".to_string()]).await?;
//!
//! // LRANGE
//! let items = client.lrange("mylist", 0, -1).await?;
//! println!("List items: {:?}", items);
//!
//! // LLEN
//! let len: i64 = client.llen("mylist").await?;
//!
//! // LINDEX
//! if let Some(item) = client.lindex("mylist", 0).await? {
//!     println!("First item: {}", item);
//! }
//!
//! // LPOP
//! if let Some(item) = client.lpop("mylist").await? {
//!     println!("Popped item: {}", item);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Set Operations
//!
//! ```no_run
//! # use redis_oxide::{Client, ConnectionConfig};
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let config = ConnectionConfig::new("redis://localhost:6379");
//! # let client = Client::connect(config).await?;
//! // SADD
//! client.sadd("myset", vec!["member1".to_string(), "member2".to_string()]).await?;
//!
//! // SMEMBERS
//! let members = client.smembers("myset").await?;
//! println!("Set members: {:?}", members);
//!
//! // SISMEMBER
//! let is_member: bool = client.sismember("myset", "member1").await?;
//!
//! // SREM
//! let removed: i64 = client.srem("myset", vec!["member1".to_string()]).await?;
//!
//! // SCARD
//! let count: i64 = client.scard("myset").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Sorted Set Operations
//!
//! ```no_run
//! # use redis_oxide::{Client, ConnectionConfig};
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let config = ConnectionConfig::new("redis://localhost:6379");
//! # let client = Client::connect(config).await?;
//! use std::collections::HashMap;
//!
//! // ZADD
//! let mut members = HashMap::new();
//! members.insert("member1".to_string(), 10.0);
//! members.insert("member2".to_string(), 20.0);
//! client.zadd("mysortedset", members).await?;
//!
//! // ZRANGE
//! let members = client.zrange("mysortedset", 0, -1).await?;
//! println!("Sorted set members: {:?}", members);
//!
//! // ZSCORE
//! if let Some(score) = client.zscore("mysortedset", "member1").await? {
//!     println!("Member1 score: {}", score);
//! }
//!
//! // ZCARD
//! let count: i64 = client.zcard("mysortedset").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## HyperLogLog Operations (Not Yet Implemented)
//!
//! HyperLogLog operations (PFADD, PFCOUNT, PFMERGE) are planned for future implementation.
//!
//! ## Geo Operations
//!
//! ```no_run,ignore
//! # use redis_oxide::{Client, ConnectionConfig};
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let config = ConnectionConfig::new("redis://localhost:6379");
//! # let client = Client::connect(config).await?;
//! ## Connection Pool Configuration
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig, PoolConfig, PoolStrategy};
//! use std::time::Duration;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut config = ConnectionConfig::new("redis://localhost:6379");
//! config.pool = PoolConfig {
//!     strategy: PoolStrategy::Pool,
//!     max_size: 20,                      // Max 20 connections
//!     min_idle: 5,                       // Keep at least 5 idle connections
//!     connection_timeout: Duration::from_secs(5),
//! };
//!
//! let client = Client::connect(config).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Multiplexed Connection (Default)
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig, PoolConfig, PoolStrategy};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut config = ConnectionConfig::new("redis://localhost:6379");
//! config.pool = PoolConfig {
//!     strategy: PoolStrategy::Multiplexed,
//!     ..Default::default()
//! };
//!
//! let client = Client::connect(config).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Transactions
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let config = ConnectionConfig::new("redis://localhost:6379");
//! # let client = Client::connect(config).await?;
//! // Start a transaction
//! let mut transaction = client.transaction().await?;
//!
//! // Add commands to the transaction (no await for adding commands)
//! transaction.set("key1", "value1");
//! transaction.set("key2", "value2");
//! transaction.incr("counter");
//!
//! // Execute the transaction
//! let results = transaction.exec().await?;
//! println!("Transaction results: {:?}", results);
//! # Ok(())
//! # }
//! ```
//!
//! ## Pipelines
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let config = ConnectionConfig::new("redis://localhost:6379");
//! # let client = Client::connect(config).await?;
//! // Create a pipeline
//! let mut pipeline = client.pipeline();
//!
//! // Add commands to the pipeline (no await for adding commands)
//! pipeline.set("key1", "value1");
//! pipeline.set("key2", "value2");
//! pipeline.get("key1");
//! pipeline.incr("counter");
//!
//! // Execute all commands at once
//! let results = pipeline.execute().await?;
//! println!("Pipeline results: {:?}", results);
//! # Ok(())
//! # }
//! ```
//!
//! ## Pub/Sub Messaging
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig};
//! use futures::StreamExt;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let config = ConnectionConfig::new("redis://localhost:6379");
//! # let client = Client::connect(config).await?;
//! // Publisher example
//! let publisher = client.publisher().await?;
//!
//! let subscribers = publisher.publish("news", "Breaking news!").await?;
//! println!("Message sent to {} subscribers", subscribers);
//!
//! // Subscriber example
//! let mut subscriber = client.subscriber().await?;
//! subscriber.subscribe(vec!["news".to_string(), "updates".to_string()]).await?;
//!
//! // Listen for messages
//! while let Some(message) = subscriber.next_message().await? {
//!     println!("Received: {} on channel {}", message.payload, message.channel);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Redis Streams
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig, StreamRange, ReadOptions};
//! use std::collections::HashMap;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let config = ConnectionConfig::new("redis://localhost:6379");
//! # let client = Client::connect(config).await?;
//! // Create and add entries to a stream
//! let mut fields = HashMap::new();
//! fields.insert("user_id".to_string(), "123".to_string());
//! fields.insert("action".to_string(), "login".to_string());
//!
//! let entry_id = client.xadd("events", "*", fields).await?;
//! println!("Added entry: {}", entry_id);
//!
//! // Read from stream with range
//! let entries = client.xrange("events", "-", "+", Some(10)).await?;
//! for entry in entries {
//!     println!("Entry {}: {:?}", entry.id, entry.fields);
//! }
//!
//! // Read from stream with options
//! let entries = client.xread(vec![("events".to_string(), "0-0".to_string())], Some(5), Some(std::time::Duration::from_millis(100))).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Consumer Groups (Redis Streams)
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let config = ConnectionConfig::new("redis://localhost:6379");
//! # let client = Client::connect(config).await?;
//! // Create a consumer group
//! client.xgroup_create("events", "processors", "$", true).await?;
//!
//! // Read from the group
//! let messages = client.xreadgroup(
//!     "processors",
//!     "worker-1",
//!     vec![("events".to_string(), ">".to_string())],
//!     Some(1),
//!     Some(std::time::Duration::from_secs(1))
//! ).await?;
//!
//! for (stream, entries) in messages {
//!     for entry in entries {
//!         println!("Processing {}: {:?}", entry.id, entry.fields);
//!         // Process the message...
//!         
//!         // Acknowledge the message
//!         client.xack("events", "processors", vec![entry.id]).await?;
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Lua Scripting
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig, Script};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let config = ConnectionConfig::new("redis://localhost:6379");
//! # let client = Client::connect(config).await?;
//! // Execute a simple Lua script
//! let script = "return redis.call('GET', KEYS[1])";
//! let result: Option<String> = client.eval(script, vec!["mykey".to_string()], vec![]).await?;
//! println!("Result: {:?}", result);
//!
//! // Script with arguments
//! let script = r#"
//!     local current = redis.call('GET', KEYS[1]) or 0
//!     local increment = tonumber(ARGV[1])
//!     local new_value = tonumber(current) + increment
//!     redis.call('SET', KEYS[1], new_value)
//!     return new_value
//! "#;
//!
//! let result: i64 = client.eval(
//!     script,
//!     vec!["counter".to_string()],
//!     vec!["5".to_string()]
//! ).await?;
//! println!("New counter value: {}", result);
//!
//! // Using Script with automatic EVALSHA caching
//! let script = Script::new(r#"
//!     local key = KEYS[1]
//!     local value = ARGV[1]
//!     redis.call('SET', key, value)
//!     return redis.call('GET', key)
//! "#);
//!
//! let result: String = script.execute(
//!     &client,
//!     vec!["mykey".to_string()],
//!     vec!["myvalue".to_string()]
//! ).await?;
//! println!("Result: {}", result);
//! # Ok(())
//! # }
//! ```
//!
//! ## Sentinel Support
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig, SentinelConfig};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let sentinel_config = SentinelConfig::new("mymaster")
//!     .add_sentinel("127.0.0.1:26379")
//!     .add_sentinel("127.0.0.1:26380")
//!     .add_sentinel("127.0.0.1:26381")
//!     .with_password("sentinel_password");
//!
//! let config = ConnectionConfig::new_with_sentinel(sentinel_config);
//! let client = Client::connect(config).await?;
//!
//! // Client automatically connects to current master
//! client.set("key", "value").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## RESP2/RESP3 Protocol Support
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig, ProtocolVersion};
//! use std::collections::HashMap;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // RESP2 (default)
//! let config_resp2 = ConnectionConfig::new("redis://localhost:6379")
//!     .with_protocol_version(ProtocolVersion::Resp2);
//! let client_resp2 = Client::connect(config_resp2).await?;
//!
//! // RESP3 (Redis 6.0+)
//! let config_resp3 = ConnectionConfig::new("redis://localhost:6379")
//!     .with_protocol_version(ProtocolVersion::Resp3);
//! let client_resp3 = Client::connect(config_resp3).await?;
//!
//! // RESP3 allows more complex data types
//! let mut map = HashMap::new();
//! map.insert("field1".to_string(), "value1".to_string());
//! map.insert("field2".to_string(), "value2".to_string());
//! client_resp3.hmset("myhash", map).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # ⚙️ Configuration Options
//!
//! ## Connection Configuration
//!
//! ```no_run,ignore
//! use redis_oxide::{ConnectionConfig, TopologyMode, ProtocolVersion};
//! use std::time::Duration;
//!
//! let config = ConnectionConfig::new("redis://localhost:6379")
//!     .with_password("secret")           // Password (optional)
//!     .with_database(0)                  // Database number
//!     .with_connect_timeout(Duration::from_secs(5))
//!     .with_operation_timeout(Duration::from_secs(30))
//!     .with_topology_mode(TopologyMode::Auto) // Auto, Standalone, or Cluster
//!     .with_protocol_version(ProtocolVersion::Resp3)  // Use RESP3 protocol
//!     .with_max_redirects(3)             // Max retries for cluster redirects
//! ```
//!
//! # 📊 Performance Features
//!
//! ## Optimized Memory Pooling
//!
//! The library includes optimized memory pooling for frequently allocated objects
//! to reduce allocation overhead and improve performance.
//!
//! ## Connection Multiplexing
//!
//! The multiplexed connection strategy shares a single connection across multiple
//! tasks using an efficient message passing system, providing excellent performance
//! for most use cases.
//!
//! ## Async/Await Support
//!
//! Fully async implementation using Tokio runtime for maximum performance and
//! resource efficiency.
//!
//! # 🧪 Testing
//!
//! To run the tests, you'll need a Redis server running on `localhost:6379`:
//!
//! ```bash
//! # Run all tests
//! cargo test
//!
//! # Run integration tests specifically
//! cargo test --test integration_tests
//!
//! # Run tests with Redis server
//! docker run --rm -p 6379:6379 redis:7
//! cargo test
//! ```
//!
//! # 📄 License
//!
//! This project is licensed under either of the following, at your option:
//!
//! - Apache License, Version 2.0, ([LICENSE-APACHE](https://github.com/nghiaphamln/redis-oxide/blob/main/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
//! - MIT license ([LICENSE-MIT](https://github.com/nghiaphamln/redis-oxide/blob/main/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

#![allow(unknown_lints)]
#![allow(clippy::unnecessary_literal_bound)]
#![deny(warnings)]
#![deny(clippy::all)]
//#![deny(clippy::pedantic)]  // Temporarily disabled due to clippy::unnecessary_literal_bound incompatibility
#![deny(clippy::nursery)]
#![allow(clippy::cargo)] // Allow cargo lints for now
#![allow(rustdoc::broken_intra_doc_links)] // Allow broken doc links for now
#![allow(rustdoc::invalid_html_tags)] // Allow invalid HTML tags for now
#![allow(clippy::needless_raw_string_hashes)] // Allow raw string hashes
#![allow(clippy::missing_panics_doc)] // Allow missing panics doc for now
#![allow(clippy::option_if_let_else)] // Allow option if let else for readability
#![allow(clippy::needless_pass_by_value)] // Allow pass by value for API design
#![allow(clippy::only_used_in_recursion)] // Allow for recursive functions
#![allow(
    clippy::similar_names,
    clippy::approx_constant,
    clippy::float_cmp,
    clippy::uninlined_format_args
)] // Allow in tests
#![allow(clippy::should_implement_trait)] // Allow for custom method names
#![allow(clippy::cast_precision_loss)] // Allow precision loss for performance metrics
#![allow(clippy::suboptimal_flops)] // Allow for readability
#![allow(clippy::single_match_else)] // Allow for readability
#![allow(clippy::redundant_pattern_matching)] // Allow for explicit patterns
#![allow(clippy::if_same_then_else)] // Allow for optimization code
#![allow(clippy::match_same_arms)] // Allow for protocol handling
#![allow(clippy::too_many_lines)] // Allow for complex protocol functions
#![allow(clippy::significant_drop_in_scrutinee)] // Allow for async code patterns
#![allow(clippy::needless_borrows_for_generic_args)] // Allow for API consistency
#![allow(clippy::map_unwrap_or)] // Allow for readability
#![allow(clippy::ignored_unit_patterns)] // Allow for explicit patterns
#![allow(clippy::manual_map)] // Allow for readability
#![allow(clippy::needless_pass_by_ref_mut)] // Allow for API consistency
#![allow(clippy::unused_self)] // Allow for trait implementations
#![warn(missing_docs)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::future_not_send)]
#![allow(clippy::significant_drop_tightening)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::implicit_clone)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::unused_async)]
#![allow(clippy::module_name_repetitions)]

pub mod client;
pub mod cluster;
pub mod commands;
pub mod connection;
pub mod pipeline;
pub mod pool;
pub mod pool_optimized;
pub mod protocol;
pub mod pubsub;
pub mod script;
pub mod sentinel;
pub mod streams;
pub mod transaction;

pub use client::Client;
pub mod core;
pub use pipeline::{Pipeline, PipelineResult};
pub use pubsub::{PubSubMessage, Publisher, Subscriber};
pub use script::{Script, ScriptManager};
pub use sentinel::{MasterInfo, SentinelClient, SentinelConfig, SentinelEndpoint};
pub use streams::{
    ConsumerGroupInfo, ConsumerInfo, PendingMessage, ReadOptions, StreamEntry, StreamInfo,
    StreamRange,
};
pub use transaction::{Transaction, TransactionResult};

pub use crate::core::{
    config::{ConnectionConfig, PoolConfig, PoolStrategy, ProtocolVersion, TopologyMode},
    error::{RedisError, RedisResult},
    types::{NodeInfo, RedisValue, SlotRange},
    value::RespValue,
};

// Re-export protocol types
pub use crate::protocol::{ProtocolNegotiation, ProtocolNegotiator, Resp3Value};
