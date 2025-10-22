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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_range_contains() {
        let range = SlotRange::new(100, 200);
        assert!(range.contains(100));
        assert!(range.contains(150));
        assert!(range.contains(200));
        assert!(!range.contains(99));
        assert!(!range.contains(201));
    }

    #[test]
    fn test_node_owns_slot() {
        let mut node = NodeInfo::new("node1".to_string(), "localhost".to_string(), 6379);
        node.slots = vec![SlotRange::new(0, 5460), SlotRange::new(10923, 16383)];

        assert!(node.owns_slot(100));
        assert!(node.owns_slot(5460));
        assert!(node.owns_slot(10923));
        assert!(node.owns_slot(16000));
        assert!(!node.owns_slot(5461));
        assert!(!node.owns_slot(10922));
    }

    #[test]
    fn test_redis_value_conversions() {
        let v1: RedisValue = "test".into();
        assert_eq!(v1, RedisValue::String("test".to_string()));

        let v2: RedisValue = 42i64.into();
        assert_eq!(v2, RedisValue::Int(42));

        let v3: RedisValue = vec![1u8, 2, 3].into();
        assert_eq!(v3, RedisValue::Bytes(vec![1, 2, 3]));
    }
}
