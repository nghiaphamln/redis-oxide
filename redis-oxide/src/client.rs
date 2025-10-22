//! High-level Redis client
//!
//! This module provides the main `Client` interface for interacting with Redis.

use crate::cluster::{calculate_slot, ClusterTopology, RedirectHandler};
use crate::commands::{
    Command, DecrByCommand, DecrCommand, DelCommand, ExistsCommand, ExpireCommand, GetCommand,
    IncrByCommand, IncrCommand, SetCommand, TtlCommand,
};
use crate::connection::{ConnectionManager, TopologyType};
use crate::pool::Pool;
use redis_oxide_core::{
    config::ConnectionConfig,
    error::{RedisError, RedisResult},
    value::RespValue,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// High-level Redis client
///
/// Automatically handles:
/// - Topology detection (Standalone vs Cluster)
/// - MOVED and ASK redirects in cluster mode
/// - Connection pooling (multiplexed or traditional)
/// - Reconnection with exponential backoff
pub struct Client {
    topology_type: TopologyType,
    config: ConnectionConfig,
    /// For standalone: single pool
    standalone_pool: Option<Arc<Pool>>,
    /// For cluster: pools per node
    cluster_pools: Arc<RwLock<HashMap<String, Arc<Pool>>>>,
    cluster_topology: Option<ClusterTopology>,
    redirect_handler: Option<RedirectHandler>,
}

impl Client {
    /// Connect to Redis with the given configuration
    ///
    /// This will automatically detect whether you're connecting to a
    /// standalone Redis server or a Redis Cluster.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use redis_oxide::{Client, ConnectionConfig};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let config = ConnectionConfig::new("redis://localhost:6379");
    ///     let client = Client::connect(config).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn connect(config: ConnectionConfig) -> RedisResult<Self> {
        info!("Connecting to Redis...");

        let mut conn_manager = ConnectionManager::new(config.clone());
        let topology_type = conn_manager.get_topology().await?;

        match topology_type {
            TopologyType::Standalone => Self::connect_standalone(config, conn_manager).await,
            TopologyType::Cluster => Self::connect_cluster(config, conn_manager).await,
        }
    }

    async fn connect_standalone(
        config: ConnectionConfig,
        _conn_manager: ConnectionManager,
    ) -> RedisResult<Self> {
        info!("Connecting to Standalone Redis");

        let endpoints = config.parse_endpoints();
        if endpoints.is_empty() {
            return Err(RedisError::Config("No endpoints specified".to_string()));
        }

        let (host, port) = endpoints[0].clone();
        let pool = Pool::new(config.clone(), host, port).await?;

        Ok(Self {
            topology_type: TopologyType::Standalone,
            config,
            standalone_pool: Some(Arc::new(pool)),
            cluster_pools: Arc::new(RwLock::new(HashMap::new())),
            cluster_topology: None,
            redirect_handler: None,
        })
    }

    async fn connect_cluster(
        config: ConnectionConfig,
        _conn_manager: ConnectionManager,
    ) -> RedisResult<Self> {
        info!("Connecting to Redis Cluster");

        let cluster_topology = ClusterTopology::new();
        let redirect_handler = RedirectHandler::new(cluster_topology.clone(), config.max_redirects);

        // Initialize cluster topology by connecting to seed nodes
        let endpoints = config.parse_endpoints();
        if endpoints.is_empty() {
            return Err(RedisError::Config("No endpoints specified".to_string()));
        }

        // Try to get cluster slots from first available node
        for (host, port) in &endpoints {
            match Pool::new(config.clone(), host.clone(), *port).await {
                Ok(pool) => {
                    // Store the initial pool
                    let node_key = format!("{}:{}", host, port);
                    let mut pools = HashMap::new();
                    pools.insert(node_key, Arc::new(pool));

                    return Ok(Self {
                        topology_type: TopologyType::Cluster,
                        config,
                        standalone_pool: None,
                        cluster_pools: Arc::new(RwLock::new(pools)),
                        cluster_topology: Some(cluster_topology),
                        redirect_handler: Some(redirect_handler),
                    });
                }
                Err(e) => {
                    warn!(
                        "Failed to connect to cluster node {}:{}: {:?}",
                        host, port, e
                    );
                    continue;
                }
            }
        }

        Err(RedisError::Cluster(
            "Failed to connect to any cluster node".to_string(),
        ))
    }

    /// Execute a command with automatic redirect handling
    async fn execute_with_redirects<C: Command>(&self, command: C) -> RedisResult<C::Output> {
        let mut retries = 0;
        let max_retries = self.config.max_redirects;

        loop {
            let result = self.execute_command_internal(&command).await;

            match result {
                Err(ref e) if e.is_redirect() && retries < max_retries => {
                    retries += 1;
                    debug!(
                        "Handling redirect (attempt {}/{}): {:?}",
                        retries, max_retries, e
                    );

                    if let Some(ref handler) = self.redirect_handler {
                        let (host, port, is_ask) = handler.handle_redirect(e).await?;

                        // Ensure we have a pool for the target node
                        self.ensure_node_pool(&host, port).await?;

                        // For ASK redirects, we need to send ASKING command first
                        if is_ask {
                            let node_key = format!("{}:{}", host, port);
                            if let Some(pool) = self.get_cluster_pool(&node_key).await {
                                // Send ASKING command
                                let _ = pool.execute_command("ASKING".to_string(), vec![]).await?;
                            }
                        }

                        // Retry the command
                        continue;
                    }
                    
                    // No redirect handler available
                    return Err(RedisError::Cluster("Redirect received but no handler available".to_string()));
                }
                Err(_e) if _e.is_redirect() => {
                    return Err(RedisError::MaxRetriesExceeded(max_retries));
                }
                other => {
                    return other
                        .map(|resp| command.parse_response(resp))
                        .and_then(|x| x)
                }
            }
        }
    }

    async fn execute_command_internal<C: Command>(&self, command: &C) -> RedisResult<RespValue> {
        match self.topology_type {
            TopologyType::Standalone => {
                if let Some(ref pool) = self.standalone_pool {
                    pool.execute_command(command.command_name().to_string(), command.args())
                        .await
                } else {
                    Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ))
                }
            }
            TopologyType::Cluster => {
                // Get the key and calculate slot
                let keys = command.keys();
                if keys.is_empty() {
                    return Err(RedisError::Cluster("Command has no keys".to_string()));
                }

                let slot = calculate_slot(keys[0]);
                debug!("Command key slot: {}", slot);

                // Try to get node from topology
                let node_key = if let Some(ref topology) = self.cluster_topology {
                    if let Some((host, port)) = topology.get_node_for_slot(slot).await {
                        Some(format!("{}:{}", host, port))
                    } else {
                        None
                    }
                } else {
                    None
                };

                // Get pool for the node
                let pool = if let Some(ref key) = node_key {
                    self.get_cluster_pool(key).await
                } else {
                    // No topology info, use any available pool
                    self.get_any_cluster_pool().await
                };

                if let Some(pool) = pool {
                    pool.execute_command(command.command_name().to_string(), command.args())
                        .await
                } else {
                    Err(RedisError::Cluster(
                        "No cluster pools available".to_string(),
                    ))
                }
            }
        }
    }

    async fn get_cluster_pool(&self, node_key: &str) -> Option<Arc<Pool>> {
        let pools = self.cluster_pools.read().await;
        pools.get(node_key).cloned()
    }

    async fn get_any_cluster_pool(&self) -> Option<Arc<Pool>> {
        let pools = self.cluster_pools.read().await;
        pools.values().next().cloned()
    }

    async fn ensure_node_pool(&self, host: &str, port: u16) -> RedisResult<()> {
        let node_key = format!("{}:{}", host, port);

        // Check if pool already exists
        {
            let pools = self.cluster_pools.read().await;
            if pools.contains_key(&node_key) {
                return Ok(());
            }
        }

        // Create new pool
        let pool = Pool::new(self.config.clone(), host.to_string(), port).await?;

        // Insert into pools
        let mut pools = self.cluster_pools.write().await;
        pools.insert(node_key, Arc::new(pool));

        Ok(())
    }

    // High-level command methods

    /// Get the value of a key
    pub async fn get(&self, key: impl Into<String>) -> RedisResult<Option<String>> {
        let command = GetCommand::new(key);
        self.execute_with_redirects(command).await
    }

    /// Set the value of a key
    pub async fn set(&self, key: impl Into<String>, value: impl Into<String>) -> RedisResult<bool> {
        let command = SetCommand::new(key, value);
        self.execute_with_redirects(command).await
    }

    /// Set the value of a key with expiration
    pub async fn set_ex(
        &self,
        key: impl Into<String>,
        value: impl Into<String>,
        expiration: Duration,
    ) -> RedisResult<bool> {
        let command = SetCommand::new(key, value).expire(expiration);
        self.execute_with_redirects(command).await
    }

    /// Set the value of a key only if it doesn't exist
    pub async fn set_nx(
        &self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> RedisResult<bool> {
        let command = SetCommand::new(key, value).only_if_not_exists();
        self.execute_with_redirects(command).await
    }

    /// Delete one or more keys
    pub async fn del(&self, keys: Vec<String>) -> RedisResult<i64> {
        let command = DelCommand::new(keys);
        self.execute_with_redirects(command).await
    }

    /// Check if one or more keys exist
    pub async fn exists(&self, keys: Vec<String>) -> RedisResult<i64> {
        let command = ExistsCommand::new(keys);
        self.execute_with_redirects(command).await
    }

    /// Set a key's time to live in seconds
    pub async fn expire(&self, key: impl Into<String>, duration: Duration) -> RedisResult<bool> {
        let command = ExpireCommand::new(key, duration);
        self.execute_with_redirects(command).await
    }

    /// Get the time to live for a key
    pub async fn ttl(&self, key: impl Into<String>) -> RedisResult<Option<i64>> {
        let command = TtlCommand::new(key);
        self.execute_with_redirects(command).await
    }

    /// Increment the integer value of a key by one
    pub async fn incr(&self, key: impl Into<String>) -> RedisResult<i64> {
        let command = IncrCommand::new(key);
        self.execute_with_redirects(command).await
    }

    /// Decrement the integer value of a key by one
    pub async fn decr(&self, key: impl Into<String>) -> RedisResult<i64> {
        let command = DecrCommand::new(key);
        self.execute_with_redirects(command).await
    }

    /// Increment the integer value of a key by the given amount
    pub async fn incr_by(&self, key: impl Into<String>, increment: i64) -> RedisResult<i64> {
        let command = IncrByCommand::new(key, increment);
        self.execute_with_redirects(command).await
    }

    /// Decrement the integer value of a key by the given amount
    pub async fn decr_by(&self, key: impl Into<String>, decrement: i64) -> RedisResult<i64> {
        let command = DecrByCommand::new(key, decrement);
        self.execute_with_redirects(command).await
    }

    /// Get the topology type
    pub fn topology_type(&self) -> TopologyType {
        self.topology_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_configuration() {
        let config = ConnectionConfig::new("redis://localhost:6379");
        assert!(!config.connection_string.is_empty());
    }
}
