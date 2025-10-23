//! High-performance async Redis client for Rust
//!
//! `redis-oxide` is a Redis client library similar to StackExchange.Redis for .NET.
//! It automatically detects whether you're connecting to a standalone Redis server
//! or a Redis Cluster, and handles MOVED/ASK redirects transparently.
//!
//! # Features
//!
//! - 🚀 **Automatic topology detection**: Auto-recognizes Standalone Redis or Redis Cluster
//! - 🔄 **MOVED/ASK redirect handling**: Automatically handles slot migrations in cluster mode
//! - 🏊 **Flexible connection strategies**: Supports both Multiplexed connections and Connection Pools
//! - 🛡️ **Type-safe command builders**: Safe API with builder pattern
//! - ⚡ **Async/await**: Fully asynchronous with Tokio runtime
//! - 🔌 **Automatic reconnection**: Reconnects with exponential backoff
//! - 📊 **Comprehensive error handling**: Detailed and clear error types
//! - ✅ **High test coverage**: Extensive unit and integration tests
//!
//! # Quick Start
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
//! ## Handling MOVED Redirects
//!
//! When encountering a MOVED error (e.g., `MOVED 9916 10.90.6.213:6002`), the library will:
//!
//! 1. ✅ Parse the error message and extract slot number and target node
//! 2. ✅ Automatically update slot mapping
//! 3. ✅ Create new connection to target node if needed
//! 4. ✅ Automatically retry the command (up to `max_redirects` times)
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ConnectionConfig::new("redis://cluster:7000")
//!     .with_max_redirects(5); // Allow up to 5 redirects
//!
//! let client = Client::connect(config).await?;
//!
//! // If encountering "MOVED 9916 10.90.6.213:6002",
//! // client automatically retries command to 10.90.6.213:6002
//! let value = client.get("mykey").await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Supported Commands
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
//! # Configuration
//!
//! ## Connection Configuration
//!
//! ```no_run
//! use redis_oxide::{ConnectionConfig, TopologyMode};
//! use std::time::Duration;
//!
//! let config = ConnectionConfig::new("redis://localhost:6379")
//!     .with_password("secret")           // Password (optional)
//!     .with_database(0)                  // Database number
//!     .with_connect_timeout(Duration::from_secs(5))
//!     .with_operation_timeout(Duration::from_secs(30))
//!     .with_topology_mode(TopologyMode::Auto) // Auto, Standalone, or Cluster
//!     .with_max_redirects(3);            // Max retries for cluster redirects
//! ```
//!
//! ## Pool Configuration
//!
//! ### Multiplexed Connection (Default)
//!
//! Uses a single connection shared between multiple tasks via mpsc channel. Suitable for most use cases.
//!
//! ```no_run
//! use redis_oxide::{ConnectionConfig, PoolConfig, PoolStrategy};
//!
//! let mut config = ConnectionConfig::new("redis://localhost:6379");
//! config.pool = PoolConfig {
//!     strategy: PoolStrategy::Multiplexed,
//!     ..Default::default()
//! };
//! ```
//!
//! ### Connection Pool
//!
//! Uses multiple connections. Suitable for very high workload with many concurrent requests.
//!
//! ```no_run
//! use redis_oxide::{ConnectionConfig, PoolConfig, PoolStrategy};
//! use std::time::Duration;
//!
//! let pool_config = PoolConfig {
//!     strategy: PoolStrategy::Pool,
//!     max_size: 20,                      // Max 20 connections
//!     min_idle: 5,                       // Keep at least 5 idle connections
//!     connection_timeout: Duration::from_secs(5),
//! };
//!
//! let mut config = ConnectionConfig::new("redis://localhost:6379");
//! config.pool = pool_config;
//! ```

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
#![allow(clippy::unnecessary_literal_bound)]

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
