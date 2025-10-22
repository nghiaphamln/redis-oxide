//! Core types and traits for redis-oxide Redis client
//!
//! This crate provides the fundamental types, traits, and error definitions
//! used throughout the redis-oxide Redis client library.

#![deny(warnings)]
#![warn(missing_docs)]

pub mod config;
pub mod error;
pub mod types;
pub mod value;

pub use config::{ConnectionConfig, PoolConfig, PoolStrategy};
pub use error::{RedisError, RedisResult};
pub use types::{RedisValue, SlotRange};
pub use value::RespValue;
