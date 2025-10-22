//! High-performance async Redis client for Rust
//!
//! `redis-oxide` is a Redis client library similar to StackExchange.Redis for .NET.
//! It automatically detects whether you're connecting to a standalone Redis server
//! or a Redis Cluster, and handles MOVED/ASK redirects transparently.
//!
//! # Features
//!
//! - Automatic topology detection (Standalone vs Cluster)
//! - Transparent handling of MOVED and ASK redirects
//! - Multiple connection strategies (multiplexed, pooled)
//! - Type-safe command builders
//! - Async/await support with Tokio
//! - Comprehensive error handling
//!
//! # Quick Start
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ConnectionConfig::new("redis://localhost:6379");
//!     let client = Client::connect(config).await?;
//!     
//!     client.set("key", "value").await?;
//!     let value: String = client.get("key").await?;
//!     println!("Value: {}", value);
//!     
//!     Ok(())
//! }
//! ```

#![deny(warnings)]
#![warn(missing_docs)]

pub mod client;
pub mod cluster;
pub mod commands;
pub mod connection;
pub mod pool;
pub mod protocol;

pub use client::Client;
pub use redis_oxide_core::{
    config::{ConnectionConfig, PoolConfig, PoolStrategy, TopologyMode},
    error::{RedisError, RedisResult},
    types::{NodeInfo, RedisValue, SlotRange},
    value::RespValue,
};
