//! Redis Cluster support
//!
//! This module provides functionality for Redis Cluster operations including:
//! - Slot calculation using CRC16
//! - MOVED and ASK redirect handling
//! - Cluster topology management
//! - Node discovery

use crc16::*;
use redis_oxide_core::{
    error::{RedisError, RedisResult},
    types::{NodeInfo, SlotRange},
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Total number of hash slots in Redis Cluster
pub const CLUSTER_SLOTS: u16 = 16384;

/// Calculate the hash slot for a given key
///
/// This implements the Redis Cluster key hashing algorithm:
/// - If the key contains {...}, only the part between the first { and first } is hashed
/// - Otherwise, the entire key is hashed
/// - The hash is CRC16 mod 16384
pub fn calculate_slot(key: &[u8]) -> u16 {
    let hash_key = extract_hash_tag(key);
    State::<XMODEM>::calculate(hash_key) % CLUSTER_SLOTS
}

/// Extract the hash tag from a key
///
/// Hash tags allow you to ensure multiple keys are allocated to the same hash slot.
/// For example: `{user1000}.following` and `{user1000}.followers` will hash to the same slot.
fn extract_hash_tag(key: &[u8]) -> &[u8] {
    // Find first '{'
    if let Some(start) = key.iter().position(|&b| b == b'{') {
        // Find first '}' after '{'
        if let Some(end) = key[start + 1..].iter().position(|&b| b == b'}') {
            let end = start + 1 + end;
            // Only use hash tag if there's at least one character between { and }
            if end > start + 1 {
                return &key[start + 1..end];
            }
        }
    }
    key
}

/// Manages cluster topology and slot mappings
#[derive(Clone)]
pub struct ClusterTopology {
    /// Mapping from slot to node address (host:port)
    slot_map: Arc<RwLock<HashMap<u16, (String, u16)>>>,
    /// Information about cluster nodes
    nodes: Arc<RwLock<HashMap<String, NodeInfo>>>,
}

impl ClusterTopology {
    /// Create a new cluster topology
    pub fn new() -> Self {
        Self {
            slot_map: Arc::new(RwLock::new(HashMap::new())),
            nodes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the node address for a given slot
    pub async fn get_node_for_slot(&self, slot: u16) -> Option<(String, u16)> {
        let slot_map = self.slot_map.read().await;
        slot_map.get(&slot).cloned()
    }

    /// Get the node address for a given key
    pub async fn get_node_for_key(&self, key: &[u8]) -> Option<(String, u16)> {
        let slot = calculate_slot(key);
        self.get_node_for_slot(slot).await
    }

    /// Update the slot mapping when a MOVED redirect occurs
    pub async fn update_slot_mapping(&self, slot: u16, host: String, port: u16) {
        let mut slot_map = self.slot_map.write().await;
        slot_map.insert(slot, (host, port));
    }

    /// Clear all slot mappings (useful for full topology refresh)
    pub async fn clear_slots(&self) {
        let mut slot_map = self.slot_map.write().await;
        slot_map.clear();
    }

    /// Update the topology from CLUSTER SLOTS response
    ///
    /// The response is an array of slot ranges, where each range is:
    /// [start_slot, end_slot, [master_host, master_port], [replica_host, replica_port], ...]
    pub async fn update_from_cluster_slots(
        &self,
        slots_data: Vec<Vec<(i64, String, i64)>>,
    ) -> RedisResult<()> {
        let mut slot_map = self.slot_map.write().await;
        let mut nodes = self.nodes.write().await;

        slot_map.clear();
        nodes.clear();

        for slot_info in slots_data {
            if slot_info.len() < 3 {
                continue;
            }

            let start_slot = slot_info[0].0 as u16;
            let end_slot = slot_info[1].0 as u16;
            let master_host = slot_info[2].1.clone();
            let master_port = slot_info[2].2 as u16;

            // Update slot map for all slots in this range
            for slot in start_slot..=end_slot {
                slot_map.insert(slot, (master_host.clone(), master_port));
            }

            // Update node information
            let node_key = format!("{}:{}", master_host, master_port);
            let mut node = NodeInfo::new(node_key.clone(), master_host, master_port);
            node.slots.push(SlotRange::new(start_slot, end_slot));
            node.is_master = true;
            nodes.insert(node_key, node);
        }

        Ok(())
    }

    /// Get all known nodes
    pub async fn get_all_nodes(&self) -> Vec<NodeInfo> {
        let nodes = self.nodes.read().await;
        nodes.values().cloned().collect()
    }

    /// Get the number of slots currently mapped
    pub async fn mapped_slots_count(&self) -> usize {
        let slot_map = self.slot_map.read().await;
        slot_map.len()
    }
}

impl Default for ClusterTopology {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to handle MOVED and ASK redirects
pub struct RedirectHandler {
    topology: ClusterTopology,
    max_redirects: usize,
}

impl RedirectHandler {
    /// Create a new redirect handler
    pub fn new(topology: ClusterTopology, max_redirects: usize) -> Self {
        Self {
            topology,
            max_redirects,
        }
    }

    /// Handle a redirect error and update topology
    pub async fn handle_redirect(&self, error: &RedisError) -> RedisResult<(String, u16, bool)> {
        match error {
            RedisError::Moved { slot, host, port } => {
                // Update topology for MOVED (permanent redirect)
                self.topology
                    .update_slot_mapping(*slot, host.clone(), *port)
                    .await;
                Ok((host.clone(), *port, false))
            }
            RedisError::Ask { host, port, .. } => {
                // ASK is temporary, don't update topology
                Ok((host.clone(), *port, true))
            }
            _ => Err(RedisError::Cluster(format!(
                "Not a redirect error: {:?}",
                error
            ))),
        }
    }

    /// Get max redirects allowed
    pub fn max_redirects(&self) -> usize {
        self.max_redirects
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_slot() {
        // Test basic key
        let slot = calculate_slot(b"mykey");
        assert!(slot < CLUSTER_SLOTS);

        // Test hash tag
        let slot1 = calculate_slot(b"{user1000}.following");
        let slot2 = calculate_slot(b"{user1000}.followers");
        assert_eq!(
            slot1, slot2,
            "Keys with same hash tag should map to same slot"
        );

        // Test different hash tags
        let _slot3 = calculate_slot(b"{user1001}.following");
        // Not guaranteed to be different, but very likely

        // Known test case from Redis spec
        let slot = calculate_slot(b"123456789");
        assert_eq!(slot, 12739);
    }

    #[test]
    fn test_extract_hash_tag() {
        assert_eq!(extract_hash_tag(b"key"), b"key");
        assert_eq!(extract_hash_tag(b"{user}key"), b"user");
        assert_eq!(extract_hash_tag(b"prefix{user}key"), b"user");
        assert_eq!(extract_hash_tag(b"{user}"), b"user");
        assert_eq!(extract_hash_tag(b"{}"), b"{}"); // Empty hash tag is ignored
        assert_eq!(extract_hash_tag(b"{"), b"{"); // No closing brace
        assert_eq!(extract_hash_tag(b"no{hash"), b"no{hash"); // No closing brace
    }

    #[tokio::test]
    async fn test_cluster_topology() {
        let topology = ClusterTopology::new();

        // Initially no mappings
        assert!(topology.get_node_for_slot(100).await.is_none());

        // Update a slot mapping
        topology
            .update_slot_mapping(100, "localhost".to_string(), 6379)
            .await;

        // Should now have the mapping
        let node = topology.get_node_for_slot(100).await;
        assert_eq!(node, Some(("localhost".to_string(), 6379)));

        // Clear and verify
        topology.clear_slots().await;
        assert!(topology.get_node_for_slot(100).await.is_none());
    }

    #[tokio::test]
    async fn test_get_node_for_key() {
        let topology = ClusterTopology::new();

        let key = b"mykey";
        let slot = calculate_slot(key);

        topology
            .update_slot_mapping(slot, "localhost".to_string(), 6379)
            .await;

        let node = topology.get_node_for_key(key).await;
        assert_eq!(node, Some(("localhost".to_string(), 6379)));
    }

    #[tokio::test]
    async fn test_redirect_handler() {
        let topology = ClusterTopology::new();
        let handler = RedirectHandler::new(topology.clone(), 3);

        // Test MOVED redirect
        let error = RedisError::Moved {
            slot: 9916,
            host: "10.90.6.213".to_string(),
            port: 6002,
        };

        let (host, port, is_ask) = handler.handle_redirect(&error).await.unwrap();
        assert_eq!(host, "10.90.6.213");
        assert_eq!(port, 6002);
        assert!(!is_ask);

        // Verify topology was updated
        let node = topology.get_node_for_slot(9916).await;
        assert_eq!(node, Some(("10.90.6.213".to_string(), 6002)));

        // Test ASK redirect
        let error = RedisError::Ask {
            slot: 100,
            host: "localhost".to_string(),
            port: 7000,
        };

        let (host, port, is_ask) = handler.handle_redirect(&error).await.unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 7000);
        assert!(is_ask);

        // Verify topology was NOT updated for ASK
        assert!(topology.get_node_for_slot(100).await.is_none());
    }
}
