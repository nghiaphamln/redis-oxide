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
//!     client.set("mykey", "myvalue").await?;
//!     let value: Option<String> = client.get("mykey").await?;
//!     println!("Value: {:?}", value);
//!     
//!     Ok(())
//! }
//! ```

#![deny(warnings)]
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
#![allow(clippy::unnecessary_literal_bound)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::implicit_clone)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::unused_async)]

pub mod client;
pub mod cluster;
pub mod commands;
pub mod connection;
pub mod pool;
pub mod protocol;

pub use client::Client;
pub mod core;

pub use crate::core::{
    config::{ConnectionConfig, PoolConfig, PoolStrategy, TopologyMode},
    error::{RedisError, RedisResult},
    types::{NodeInfo, RedisValue, SlotRange},
    value::RespValue,
};
