//! Async Redis client for Rust.
//!
//! `redis-oxide` supports standalone Redis, Redis Cluster, and Redis Sentinel
//! with Tokio-based connections, pooling, pipelines, transactions, Pub/Sub,
//! Streams, Lua scripts, and RESP2 or RESP3.
//!
//! # Quick start
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = Client::connect(ConnectionConfig::new("redis://localhost:6379")).await?;
//!     client.set("greeting", "hello").await?;
//!     println!("{:?}", client.get("greeting").await?);
//!     Ok(())
//! }
//! ```
//!
//! # Compatibility
//!
//! - Rust 1.82 or newer.
//! - Redis 6.0 or newer.
//! - Tokio on Linux, macOS, and Windows.
//! - TLS is not implemented; `rediss://` URLs are rejected.
//!
//! # Guides
//!
//! - [Getting started](https://github.com/nghiaphamln/redis-oxide/blob/main/docs/getting-started.md)
//! - [Deployment and topologies](https://github.com/nghiaphamln/redis-oxide/blob/main/docs/deployment.md)
//! - [Troubleshooting](https://github.com/nghiaphamln/redis-oxide/blob/main/docs/troubleshooting.md)
//! - [Migrating to 0.3](https://github.com/nghiaphamln/redis-oxide/blob/main/docs/migration-0.3.md)
//!
//! # License
//!
//! Licensed under the MIT License.

#![deny(warnings)]
#![warn(missing_docs)]

pub mod client;
pub mod cluster;
pub mod commands;
pub mod connection;
pub mod core;
pub mod pipeline;
pub mod pool;
pub mod protocol;
pub mod pubsub;
pub mod script;
pub mod sentinel;
pub mod streams;
pub mod transaction;

pub use crate::core::{
    config::{ConnectionConfig, PoolConfig, PoolStrategy, ProtocolVersion, TopologyMode},
    error::{RedisError, RedisResult},
    types::{NodeInfo, RedisValue, SlotRange},
    value::RespValue,
};
pub use client::Client;
pub use pipeline::{Pipeline, PipelineResult};
pub use protocol::Resp3Value;
pub use pubsub::{PubSubMessage, Publisher, Subscriber};
pub use script::{Script, ScriptManager};
pub use sentinel::{MasterInfo, SentinelClient, SentinelConfig, SentinelEndpoint};
pub use streams::{
    ConsumerGroupInfo, ConsumerInfo, PendingMessage, ReadOptions, StreamEntry, StreamInfo,
    StreamRange,
};
pub use transaction::{Transaction, TransactionResult};
