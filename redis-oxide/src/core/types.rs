//! Common types used throughout the library

use serde::{Deserialize, Serialize};

/// Represents a value that can be stored in Redis
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedisValue {
    /// Null value
    Nil,
    /// String value
    String(String),
    /// Binary data
    Bytes(Vec<u8>),
    /// Integer value
    Int(i64),
    /// Array of values
    Array(Vec<RedisValue>),
}

impl From<String> for RedisValue {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for RedisValue {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<Vec<u8>> for RedisValue {
    fn from(b: Vec<u8>) -> Self {
        Self::Bytes(b)
    }
}

impl From<i64> for RedisValue {
    fn from(i: i64) -> Self {
        Self::Int(i)
    }
}

impl From<i32> for RedisValue {
    fn from(i: i32) -> Self {
        Self::Int(i64::from(i))
    }
}

impl From<Vec<Self>> for RedisValue {
    fn from(arr: Vec<Self>) -> Self {
        Self::Array(arr)
    }
}

/// Represents a slot range in a Redis cluster
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotRange {
    /// Start of the slot range (inclusive)
    pub start: u16,
    /// End of the slot range (inclusive)
    pub end: u16,
}

impl SlotRange {
    /// Create a new slot range
    #[must_use]
    pub const fn new(start: u16, end: u16) -> Self {
        Self { start, end }
    }

    /// Check if a slot is within this range
    #[must_use]
    pub const fn contains(&self, slot: u16) -> bool {
        slot >= self.start && slot <= self.end
    }
}

/// Node information in a Redis cluster
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Node ID
    pub id: String,
    /// Host address
    pub host: String,
    /// Port number
    pub port: u16,
    /// Slot ranges owned by this node
    pub slots: Vec<SlotRange>,
    /// Whether this is a master node
    pub is_master: bool,
}

impl NodeInfo {
    /// Create a new node info
    #[must_use]
    pub const fn new(id: String, host: String, port: u16) -> Self {
        Self {
            id,
            host,
            port,
            slots: Vec::new(),
            is_master: true,
        }
    }

    /// Check if this node owns a given slot
    #[must_use]
    pub fn owns_slot(&self, slot: u16) -> bool {
        self.slots.iter().any(|range| range.contains(slot))
    }
}
