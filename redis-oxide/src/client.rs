//! High-level Redis client
//!
//! This module provides the main `Client` interface for interacting with Redis.

use crate::cluster::{calculate_slot, ClusterTopology, RedirectHandler};
use crate::commands::{
    Command,
    DecrByCommand,
    DecrCommand,
    DelCommand,
    ExistsCommand,
    ExpireCommand,
    GetCommand,
    HDelCommand,
    HExistsCommand,
    HGetAllCommand,
    HGetCommand,
    HLenCommand,
    HMGetCommand,
    HMSetCommand,
    HSetCommand,
    IncrByCommand,
    IncrCommand,
    // List commands
    LIndexCommand,
    LLenCommand,
    LPopCommand,
    LPushCommand,
    LRangeCommand,
    LSetCommand,
    RPopCommand,
    RPushCommand,
    // Set commands
    SAddCommand,
    SCardCommand,
    SIsMemberCommand,
    SMembersCommand,
    SPopCommand,
    SRandMemberCommand,
    SRemCommand,
    SetCommand,
    TtlCommand,
    // Sorted Set commands
    ZAddCommand,
    ZCardCommand,
    ZRangeCommand,
    ZRankCommand,
    ZRemCommand,
    ZRevRankCommand,
    ZScoreCommand,
};
use crate::connection::{ConnectionManager, TopologyType};
use crate::core::{
    config::ConnectionConfig,
    error::{RedisError, RedisResult},
    value::RespValue,
};
use crate::pipeline::{Pipeline, PipelineCommand, PipelineExecutor};
use crate::pool::Pool;
use crate::pubsub::{PubSubConnection, Publisher, Subscriber};
use crate::transaction::{Transaction, TransactionCommand, TransactionExecutor};
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
#[derive(Clone)]
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
                    // Try next node
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
                    return Err(RedisError::Cluster(
                        "Redirect received but no handler available".to_string(),
                    ));
                }
                Err(e) if e.is_redirect() => {
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

    // Hash operations

    /// Get the value of a hash field
    pub async fn hget(
        &self,
        key: impl Into<String>,
        field: impl Into<String>,
    ) -> RedisResult<Option<String>> {
        let command = HGetCommand::new(key, field);
        self.execute_with_redirects(command).await
    }

    /// Set the value of a hash field
    pub async fn hset(
        &self,
        key: impl Into<String>,
        field: impl Into<String>,
        value: impl Into<String>,
    ) -> RedisResult<i64> {
        let command = HSetCommand::new(key, field, value);
        self.execute_with_redirects(command).await
    }

    /// Delete one or more hash fields
    pub async fn hdel(&self, key: impl Into<String>, fields: Vec<String>) -> RedisResult<i64> {
        let command = HDelCommand::new(key, fields);
        self.execute_with_redirects(command).await
    }

    /// Get all fields and values in a hash
    pub async fn hgetall(
        &self,
        key: impl Into<String>,
    ) -> RedisResult<std::collections::HashMap<String, String>> {
        let command = HGetAllCommand::new(key);
        self.execute_with_redirects(command).await
    }

    /// Get the values of multiple hash fields
    pub async fn hmget(
        &self,
        key: impl Into<String>,
        fields: Vec<String>,
    ) -> RedisResult<Vec<Option<String>>> {
        let command = HMGetCommand::new(key, fields);
        self.execute_with_redirects(command).await
    }

    /// Set multiple hash fields to multiple values
    pub async fn hmset(
        &self,
        key: impl Into<String>,
        fields: std::collections::HashMap<String, String>,
    ) -> RedisResult<String> {
        let command = HMSetCommand::new(key, fields);
        self.execute_with_redirects(command).await
    }

    /// Get the number of fields in a hash
    pub async fn hlen(&self, key: impl Into<String>) -> RedisResult<i64> {
        let command = HLenCommand::new(key);
        self.execute_with_redirects(command).await
    }

    /// Determine if a hash field exists
    pub async fn hexists(
        &self,
        key: impl Into<String>,
        field: impl Into<String>,
    ) -> RedisResult<bool> {
        let command = HExistsCommand::new(key, field);
        self.execute_with_redirects(command).await
    }

    // List operations

    /// Push one or more values to the head of a list
    pub async fn lpush(&self, key: impl Into<String>, values: Vec<String>) -> RedisResult<i64> {
        let command = LPushCommand::new(key, values);
        self.execute_with_redirects(command).await
    }

    /// Push one or more values to the tail of a list
    pub async fn rpush(&self, key: impl Into<String>, values: Vec<String>) -> RedisResult<i64> {
        let command = RPushCommand::new(key, values);
        self.execute_with_redirects(command).await
    }

    /// Remove and return the first element of a list
    pub async fn lpop(&self, key: impl Into<String>) -> RedisResult<Option<String>> {
        let command = LPopCommand::new(key);
        self.execute_with_redirects(command).await
    }

    /// Remove and return the last element of a list
    pub async fn rpop(&self, key: impl Into<String>) -> RedisResult<Option<String>> {
        let command = RPopCommand::new(key);
        self.execute_with_redirects(command).await
    }

    /// Get a range of elements from a list
    pub async fn lrange(
        &self,
        key: impl Into<String>,
        start: i64,
        stop: i64,
    ) -> RedisResult<Vec<String>> {
        let command = LRangeCommand::new(key, start, stop);
        self.execute_with_redirects(command).await
    }

    /// Get the length of a list
    pub async fn llen(&self, key: impl Into<String>) -> RedisResult<i64> {
        let command = LLenCommand::new(key);
        self.execute_with_redirects(command).await
    }

    /// Get an element from a list by its index
    pub async fn lindex(&self, key: impl Into<String>, index: i64) -> RedisResult<Option<String>> {
        let command = LIndexCommand::new(key, index);
        self.execute_with_redirects(command).await
    }

    /// Set the value of an element in a list by its index
    pub async fn lset(
        &self,
        key: impl Into<String>,
        index: i64,
        value: impl Into<String>,
    ) -> RedisResult<()> {
        let command = LSetCommand::new(key, index, value);
        let _result: String = self.execute_with_redirects(command).await?;
        Ok(())
    }

    // Set operations

    /// Add one or more members to a set
    pub async fn sadd(&self, key: impl Into<String>, members: Vec<String>) -> RedisResult<i64> {
        let command = SAddCommand::new(key, members);
        self.execute_with_redirects(command).await
    }

    /// Remove one or more members from a set
    pub async fn srem(&self, key: impl Into<String>, members: Vec<String>) -> RedisResult<i64> {
        let command = SRemCommand::new(key, members);
        self.execute_with_redirects(command).await
    }

    /// Get all members of a set
    pub async fn smembers(
        &self,
        key: impl Into<String>,
    ) -> RedisResult<std::collections::HashSet<String>> {
        let command = SMembersCommand::new(key);
        self.execute_with_redirects(command).await
    }

    /// Determine if a member is in a set
    pub async fn sismember(
        &self,
        key: impl Into<String>,
        member: impl Into<String>,
    ) -> RedisResult<bool> {
        let command = SIsMemberCommand::new(key, member);
        self.execute_with_redirects(command).await
    }

    /// Get the number of members in a set
    pub async fn scard(&self, key: impl Into<String>) -> RedisResult<i64> {
        let command = SCardCommand::new(key);
        self.execute_with_redirects(command).await
    }

    /// Remove and return a random member from a set
    pub async fn spop(&self, key: impl Into<String>) -> RedisResult<Option<String>> {
        let command = SPopCommand::new(key);
        self.execute_with_redirects(command).await
    }

    /// Get a random member from a set
    pub async fn srandmember(&self, key: impl Into<String>) -> RedisResult<Option<String>> {
        let command = SRandMemberCommand::new(key);
        self.execute_with_redirects(command).await
    }

    // Sorted Set operations

    /// Add one or more members to a sorted set
    pub async fn zadd(
        &self,
        key: impl Into<String>,
        members: std::collections::HashMap<String, f64>,
    ) -> RedisResult<i64> {
        let command = ZAddCommand::new(key, members);
        self.execute_with_redirects(command).await
    }

    /// Remove one or more members from a sorted set
    pub async fn zrem(&self, key: impl Into<String>, members: Vec<String>) -> RedisResult<i64> {
        let command = ZRemCommand::new(key, members);
        self.execute_with_redirects(command).await
    }

    /// Get a range of members from a sorted set by index
    pub async fn zrange(
        &self,
        key: impl Into<String>,
        start: i64,
        stop: i64,
    ) -> RedisResult<Vec<String>> {
        let command = ZRangeCommand::new(key, start, stop);
        self.execute_with_redirects(command).await
    }

    /// Get the score of a member in a sorted set
    pub async fn zscore(
        &self,
        key: impl Into<String>,
        member: impl Into<String>,
    ) -> RedisResult<Option<f64>> {
        let command = ZScoreCommand::new(key, member);
        self.execute_with_redirects(command).await
    }

    /// Get the number of members in a sorted set
    pub async fn zcard(&self, key: impl Into<String>) -> RedisResult<i64> {
        let command = ZCardCommand::new(key);
        self.execute_with_redirects(command).await
    }

    /// Get the rank of a member in a sorted set (lowest to highest)
    pub async fn zrank(
        &self,
        key: impl Into<String>,
        member: impl Into<String>,
    ) -> RedisResult<Option<i64>> {
        let command = ZRankCommand::new(key, member);
        self.execute_with_redirects(command).await
    }

    /// Get the rank of a member in a sorted set (highest to lowest)
    pub async fn zrevrank(
        &self,
        key: impl Into<String>,
        member: impl Into<String>,
    ) -> RedisResult<Option<i64>> {
        let command = ZRevRankCommand::new(key, member);
        self.execute_with_redirects(command).await
    }

    /// Create a new pipeline for batching commands
    ///
    /// Pipeline allows you to send multiple commands to Redis in a single
    /// network round-trip, which can significantly improve performance.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use redis_oxide::{Client, ConnectionConfig};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ConnectionConfig::new("redis://localhost:6379");
    /// let client = Client::connect(config).await?;
    ///
    /// let mut pipeline = client.pipeline();
    /// pipeline.set("key1", "value1");
    /// pipeline.set("key2", "value2");
    /// pipeline.get("key1");
    ///
    /// let results = pipeline.execute().await?;
    /// println!("Pipeline results: {:?}", results);
    /// # Ok(())
    /// # }
    /// ```
    pub fn pipeline(&self) -> Pipeline {
        let client_executor = ClientPipelineExecutor {
            client: self.clone(),
        };
        Pipeline::new(Arc::new(tokio::sync::Mutex::new(client_executor)))
    }

    /// Create a new transaction for atomic command execution
    ///
    /// Transactions allow you to execute multiple commands atomically using
    /// MULTI/EXEC. You can also use WATCH to monitor keys for changes.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use redis_oxide::{Client, ConnectionConfig};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ConnectionConfig::new("redis://localhost:6379");
    /// let client = Client::connect(config).await?;
    ///
    /// let mut transaction = client.transaction().await?;
    /// transaction.set("key1", "value1");
    /// transaction.set("key2", "value2");
    /// transaction.incr("counter");
    ///
    /// let results = transaction.exec().await?;
    /// println!("Transaction results: {:?}", results);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn transaction(&self) -> RedisResult<Transaction> {
        let client_executor = ClientTransactionExecutor {
            client: self.clone(),
        };
        Ok(Transaction::new(Arc::new(tokio::sync::Mutex::new(
            client_executor,
        ))))
    }

    /// Publish a message to a Redis channel
    ///
    /// Returns the number of subscribers that received the message.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use redis_oxide::{Client, ConnectionConfig};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ConnectionConfig::new("redis://localhost:6379");
    /// let client = Client::connect(config).await?;
    ///
    /// let subscribers = client.publish("news", "Breaking news!").await?;
    /// println!("Message sent to {} subscribers", subscribers);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn publish(
        &self,
        channel: impl Into<String>,
        message: impl Into<String>,
    ) -> RedisResult<i64> {
        let channel = channel.into();
        let message = message.into();

        let args = vec![
            RespValue::from(channel.as_str()),
            RespValue::from(message.as_str()),
        ];

        match self.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.standalone_pool {
                    let result = pool.execute_command("PUBLISH".to_string(), args).await?;
                    result.as_int()
                } else {
                    Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ))
                }
            }
            TopologyType::Cluster => {
                // For cluster, use any available node for PUBLISH
                let pools = self.cluster_pools.read().await;
                if let Some((_, pool)) = pools.iter().next() {
                    let result = pool.execute_command("PUBLISH".to_string(), args).await?;
                    result.as_int()
                } else {
                    Err(RedisError::Cluster(
                        "No cluster nodes available".to_string(),
                    ))
                }
            }
        }
    }

    /// Create a new subscriber for receiving messages from Redis channels
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use redis_oxide::{Client, ConnectionConfig};
    /// use futures::StreamExt;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ConnectionConfig::new("redis://localhost:6379");
    /// let client = Client::connect(config).await?;
    ///
    /// let mut subscriber = client.subscriber().await?;
    /// subscriber.subscribe(vec!["news".to_string()]).await?;
    ///
    /// while let Some(message) = subscriber.next_message().await? {
    ///     println!("Received: {} on {}", message.payload, message.channel);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn subscriber(&self) -> RedisResult<Subscriber> {
        let client_connection = ClientPubSubConnection {
            client: self.clone(),
        };
        Ok(Subscriber::new(Arc::new(tokio::sync::Mutex::new(
            client_connection,
        ))))
    }

    /// Create a new publisher for sending messages to Redis channels
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use redis_oxide::{Client, ConnectionConfig};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ConnectionConfig::new("redis://localhost:6379");
    /// let client = Client::connect(config).await?;
    ///
    /// let publisher = client.publisher().await?;
    /// let subscribers = publisher.publish("news", "Breaking news!").await?;
    /// println!("Message sent to {} subscribers", subscribers);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn publisher(&self) -> RedisResult<Publisher> {
        let client_connection = ClientPubSubConnection {
            client: self.clone(),
        };
        Ok(Publisher::new(Arc::new(tokio::sync::Mutex::new(
            client_connection,
        ))))
    }

    // Lua scripting methods

    /// Execute a Lua script using EVAL
    ///
    /// # Arguments
    ///
    /// * `script` - The Lua script source code
    /// * `keys` - List of Redis keys that the script will access (KEYS array in Lua)
    /// * `args` - List of arguments to pass to the script (ARGV array in Lua)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use redis_oxide::{Client, ConnectionConfig};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ConnectionConfig::new("redis://localhost:6379");
    /// let client = Client::connect(config).await?;
    ///
    /// let script = "return redis.call('GET', KEYS[1])";
    /// let result: Option<String> = client.eval(
    ///     script,
    ///     vec!["mykey".to_string()],
    ///     vec![]
    /// ).await?;
    /// println!("Result: {:?}", result);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn eval<T>(
        &self,
        script: &str,
        keys: Vec<String>,
        args: Vec<String>,
    ) -> RedisResult<T>
    where
        T: std::convert::TryFrom<RespValue>,
        T::Error: Into<RedisError>,
    {
        let mut cmd_args = vec![
            RespValue::from(script),
            RespValue::from(keys.len().to_string()),
        ];

        // Add keys
        for key in keys {
            cmd_args.push(RespValue::from(key));
        }

        // Add arguments
        for arg in args {
            cmd_args.push(RespValue::from(arg));
        }

        let result = match self.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.standalone_pool {
                    pool.execute_command("EVAL".to_string(), cmd_args).await?
                } else {
                    return Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ));
                }
            }
            TopologyType::Cluster => {
                // For cluster, use any available node for script execution
                let pools = self.cluster_pools.read().await;
                if let Some((_, pool)) = pools.iter().next() {
                    pool.execute_command("EVAL".to_string(), cmd_args).await?
                } else {
                    return Err(RedisError::Cluster(
                        "No cluster nodes available".to_string(),
                    ));
                }
            }
        };

        T::try_from(result).map_err(Into::into)
    }

    /// Execute a Lua script using EVALSHA (script must be cached)
    ///
    /// # Arguments
    ///
    /// * `sha` - The SHA1 hash of the script
    /// * `keys` - List of Redis keys that the script will access (KEYS array in Lua)
    /// * `args` - List of arguments to pass to the script (ARGV array in Lua)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use redis_oxide::{Client, ConnectionConfig};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ConnectionConfig::new("redis://localhost:6379");
    /// let client = Client::connect(config).await?;
    ///
    /// // First load the script
    /// let script = "return redis.call('GET', KEYS[1])";
    /// let sha = client.script_load(script).await?;
    ///
    /// // Then execute using SHA
    /// let result: Option<String> = client.evalsha(
    ///     &sha,
    ///     vec!["mykey".to_string()],
    ///     vec![]
    /// ).await?;
    /// println!("Result: {:?}", result);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn evalsha<T>(
        &self,
        sha: &str,
        keys: Vec<String>,
        args: Vec<String>,
    ) -> RedisResult<T>
    where
        T: std::convert::TryFrom<RespValue>,
        T::Error: Into<RedisError>,
    {
        let mut cmd_args = vec![
            RespValue::from(sha),
            RespValue::from(keys.len().to_string()),
        ];

        // Add keys
        for key in keys {
            cmd_args.push(RespValue::from(key));
        }

        // Add arguments
        for arg in args {
            cmd_args.push(RespValue::from(arg));
        }

        let result = match self.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.standalone_pool {
                    pool.execute_command("EVALSHA".to_string(), cmd_args)
                        .await?
                } else {
                    return Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ));
                }
            }
            TopologyType::Cluster => {
                // For cluster, use any available node for script execution
                let pools = self.cluster_pools.read().await;
                if let Some((_, pool)) = pools.iter().next() {
                    pool.execute_command("EVALSHA".to_string(), cmd_args)
                        .await?
                } else {
                    return Err(RedisError::Cluster(
                        "No cluster nodes available".to_string(),
                    ));
                }
            }
        };

        T::try_from(result).map_err(Into::into)
    }

    /// Load a Lua script into Redis cache
    ///
    /// Returns the SHA1 hash of the script that can be used with EVALSHA.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use redis_oxide::{Client, ConnectionConfig};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ConnectionConfig::new("redis://localhost:6379");
    /// let client = Client::connect(config).await?;
    ///
    /// let script = "return 'Hello, World!'";
    /// let sha = client.script_load(script).await?;
    /// println!("Script loaded with SHA: {}", sha);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn script_load(&self, script: &str) -> RedisResult<String> {
        let result = match self.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.standalone_pool {
                    pool.execute_command(
                        "SCRIPT".to_string(),
                        vec![RespValue::from("LOAD"), RespValue::from(script)],
                    )
                    .await?
                } else {
                    return Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ));
                }
            }
            TopologyType::Cluster => {
                // For cluster, load script on all nodes
                let pools = self.cluster_pools.read().await;
                let mut sha = String::new();

                for (_, pool) in pools.iter() {
                    let result = pool
                        .execute_command(
                            "SCRIPT".to_string(),
                            vec![RespValue::from("LOAD"), RespValue::from(script)],
                        )
                        .await?;
                    sha = result.as_string()?;
                }

                if sha.is_empty() {
                    return Err(RedisError::Cluster(
                        "No cluster nodes available".to_string(),
                    ));
                }

                return Ok(sha);
            }
        };

        result.as_string()
    }

    /// Check if scripts exist in Redis cache
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use redis_oxide::{Client, ConnectionConfig};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ConnectionConfig::new("redis://localhost:6379");
    /// let client = Client::connect(config).await?;
    ///
    /// let script = "return 'Hello'";
    /// let sha = client.script_load(script).await?;
    ///
    /// let exists = client.script_exists(vec![sha]).await?;
    /// println!("Script exists: {:?}", exists);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn script_exists(&self, shas: Vec<String>) -> RedisResult<Vec<bool>> {
        let mut cmd_args = vec![RespValue::from("EXISTS")];
        for sha in shas {
            cmd_args.push(RespValue::from(sha));
        }

        let result = match self.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.standalone_pool {
                    pool.execute_command("SCRIPT".to_string(), cmd_args).await?
                } else {
                    return Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ));
                }
            }
            TopologyType::Cluster => {
                // For cluster, check on any available node
                let pools = self.cluster_pools.read().await;
                if let Some((_, pool)) = pools.iter().next() {
                    pool.execute_command("SCRIPT".to_string(), cmd_args).await?
                } else {
                    return Err(RedisError::Cluster(
                        "No cluster nodes available".to_string(),
                    ));
                }
            }
        };

        match result {
            RespValue::Array(items) => {
                let mut exists = Vec::new();
                for item in items {
                    match item {
                        RespValue::Integer(1) => exists.push(true),
                        RespValue::Integer(0) => exists.push(false),
                        _ => {
                            return Err(RedisError::Type(format!(
                                "Unexpected response in SCRIPT EXISTS: {:?}",
                                item
                            )))
                        }
                    }
                }
                Ok(exists)
            }
            _ => Err(RedisError::Type(format!(
                "Unexpected response type for SCRIPT EXISTS: {:?}",
                result
            ))),
        }
    }

    /// Flush all scripts from Redis cache
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use redis_oxide::{Client, ConnectionConfig};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ConnectionConfig::new("redis://localhost:6379");
    /// let client = Client::connect(config).await?;
    ///
    /// client.script_flush().await?;
    /// println!("All scripts flushed from cache");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn script_flush(&self) -> RedisResult<()> {
        let cmd_args = vec![RespValue::from("FLUSH")];

        match self.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.standalone_pool {
                    let _result = pool.execute_command("SCRIPT".to_string(), cmd_args).await?;
                    Ok(())
                } else {
                    Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ))
                }
            }
            TopologyType::Cluster => {
                // For cluster, flush scripts on all nodes
                let pools = self.cluster_pools.read().await;
                for (_, pool) in pools.iter() {
                    let _result = pool
                        .execute_command("SCRIPT".to_string(), cmd_args.clone())
                        .await?;
                }
                Ok(())
            }
        }
    }

    // Redis Streams methods

    /// Add an entry to a stream using XADD
    ///
    /// # Arguments
    ///
    /// * `stream` - The name of the stream
    /// * `id` - The entry ID ("*" for auto-generation)
    /// * `fields` - The field-value pairs for the entry
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use redis_oxide::{Client, ConnectionConfig};
    /// use std::collections::HashMap;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ConnectionConfig::new("redis://localhost:6379");
    /// let client = Client::connect(config).await?;
    ///
    /// let mut fields = HashMap::new();
    /// fields.insert("user_id".to_string(), "123".to_string());
    /// fields.insert("action".to_string(), "login".to_string());
    ///
    /// let entry_id = client.xadd("events", "*", fields).await?;
    /// println!("Added entry: {}", entry_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn xadd(
        &self,
        stream: impl Into<String>,
        id: impl Into<String>,
        fields: std::collections::HashMap<String, String>,
    ) -> RedisResult<String> {
        let stream = stream.into();
        let id = id.into();

        let mut cmd_args = vec![RespValue::from(stream.clone()), RespValue::from(id)];

        // Add field-value pairs
        for (field, value) in fields {
            cmd_args.push(RespValue::from(field));
            cmd_args.push(RespValue::from(value));
        }

        let result = match self.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.standalone_pool {
                    pool.execute_command("XADD".to_string(), cmd_args).await?
                } else {
                    return Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ));
                }
            }
            TopologyType::Cluster => {
                // For cluster, use the stream name to determine the slot
                let slot = calculate_slot(stream.as_bytes());

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

                if let Some(node_key) = node_key {
                    if let Some(pool) = self.get_cluster_pool(&node_key).await {
                        pool.execute_command("XADD".to_string(), cmd_args).await?
                    } else {
                        return Err(RedisError::Cluster(format!(
                            "Pool not found for node: {}",
                            node_key
                        )));
                    }
                } else {
                    return Err(RedisError::Cluster(format!(
                        "No node found for slot: {}",
                        slot
                    )));
                }
            }
        };

        result.as_string()
    }

    /// Read entries from one or more streams using XREAD
    ///
    /// # Arguments
    ///
    /// * `streams` - Vector of (stream_name, last_id) pairs
    /// * `count` - Maximum number of entries per stream (None for no limit)
    /// * `block` - Block timeout (None for non-blocking)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use redis_oxide::{Client, ConnectionConfig};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ConnectionConfig::new("redis://localhost:6379");
    /// let client = Client::connect(config).await?;
    ///
    /// // Non-blocking read
    /// let streams = vec![("events".to_string(), "$".to_string())];
    /// let messages = client.xread(streams, Some(10), None).await?;
    ///
    /// for (stream, entries) in messages {
    ///     for entry in entries {
    ///         println!("Stream {}: {} -> {:?}", stream, entry.id, entry.fields);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn xread(
        &self,
        streams: Vec<(String, String)>,
        count: Option<u64>,
        block: Option<Duration>,
    ) -> RedisResult<std::collections::HashMap<String, Vec<crate::streams::StreamEntry>>> {
        let mut cmd_args = vec![];

        // Add COUNT option
        if let Some(count) = count {
            cmd_args.push(RespValue::from("COUNT"));
            cmd_args.push(RespValue::from(count.to_string()));
        }

        // Add BLOCK option
        if let Some(block) = block {
            cmd_args.push(RespValue::from("BLOCK"));
            cmd_args.push(RespValue::from(block.as_millis().to_string()));
        }

        // Add STREAMS keyword
        cmd_args.push(RespValue::from("STREAMS"));

        // Add stream names
        for (stream, _) in &streams {
            cmd_args.push(RespValue::from(stream.clone()));
        }

        // Add stream IDs
        for (_, id) in &streams {
            cmd_args.push(RespValue::from(id.clone()));
        }

        let result = match self.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.standalone_pool {
                    pool.execute_command("XREAD".to_string(), cmd_args).await?
                } else {
                    return Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ));
                }
            }
            TopologyType::Cluster => {
                // For cluster, use any available node (XREAD can read from multiple streams)
                let pools = self.cluster_pools.read().await;
                if let Some((_, pool)) = pools.iter().next() {
                    pool.execute_command("XREAD".to_string(), cmd_args).await?
                } else {
                    return Err(RedisError::Cluster(
                        "No cluster nodes available".to_string(),
                    ));
                }
            }
        };

        crate::streams::parse_xread_response(result)
    }

    /// Read entries from a stream range using XRANGE
    ///
    /// # Arguments
    ///
    /// * `stream` - The name of the stream
    /// * `start` - Start ID (inclusive, "-" for beginning)
    /// * `end` - End ID (inclusive, "+" for end)
    /// * `count` - Maximum number of entries to return
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use redis_oxide::{Client, ConnectionConfig};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ConnectionConfig::new("redis://localhost:6379");
    /// let client = Client::connect(config).await?;
    ///
    /// // Get all entries
    /// let entries = client.xrange("events", "-", "+", None).await?;
    /// for entry in entries {
    ///     println!("Entry {}: {:?}", entry.id, entry.fields);
    /// }
    ///
    /// // Get last 10 entries
    /// let recent = client.xrange("events", "-", "+", Some(10)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn xrange(
        &self,
        stream: impl Into<String>,
        start: impl Into<String>,
        end: impl Into<String>,
        count: Option<u64>,
    ) -> RedisResult<Vec<crate::streams::StreamEntry>> {
        let stream = stream.into();
        let mut cmd_args = vec![
            RespValue::from(stream.clone()),
            RespValue::from(start.into()),
            RespValue::from(end.into()),
        ];

        if let Some(count) = count {
            cmd_args.push(RespValue::from("COUNT"));
            cmd_args.push(RespValue::from(count.to_string()));
        }

        let result = match self.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.standalone_pool {
                    pool.execute_command("XRANGE".to_string(), cmd_args).await?
                } else {
                    return Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ));
                }
            }
            TopologyType::Cluster => {
                // For cluster, use the stream name to determine the slot
                let slot = calculate_slot(stream.as_bytes());
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

                if let Some(node_key) = node_key {
                    if let Some(pool) = self.get_cluster_pool(&node_key).await {
                        pool.execute_command("XRANGE".to_string(), cmd_args).await?
                    } else {
                        return Err(RedisError::Cluster(format!(
                            "Pool not found for node: {}",
                            node_key
                        )));
                    }
                } else {
                    return Err(RedisError::Cluster(format!(
                        "No node found for slot: {}",
                        slot
                    )));
                }
            }
        };

        crate::streams::parse_stream_entries(result)
    }

    /// Get the length of a stream using XLEN
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use redis_oxide::{Client, ConnectionConfig};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ConnectionConfig::new("redis://localhost:6379");
    /// let client = Client::connect(config).await?;
    ///
    /// let length = client.xlen("events").await?;
    /// println!("Stream has {} entries", length);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn xlen(&self, stream: impl Into<String>) -> RedisResult<u64> {
        let stream = stream.into();
        let cmd_args = vec![RespValue::from(stream.clone())];

        let result = match self.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.standalone_pool {
                    pool.execute_command("XLEN".to_string(), cmd_args).await?
                } else {
                    return Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ));
                }
            }
            TopologyType::Cluster => {
                // For cluster, use the stream name to determine the slot
                let slot = calculate_slot(stream.as_bytes());
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

                if let Some(node_key) = node_key {
                    if let Some(pool) = self.get_cluster_pool(&node_key).await {
                        pool.execute_command("XLEN".to_string(), cmd_args).await?
                    } else {
                        return Err(RedisError::Cluster(format!(
                            "Pool not found for node: {}",
                            node_key
                        )));
                    }
                } else {
                    return Err(RedisError::Cluster(format!(
                        "No node found for slot: {}",
                        slot
                    )));
                }
            }
        };

        Ok(result.as_int()? as u64)
    }

    /// Create a consumer group using XGROUP CREATE
    ///
    /// # Arguments
    ///
    /// * `stream` - The name of the stream
    /// * `group` - The name of the consumer group
    /// * `id` - The starting ID for the group ("$" for latest, "0" for beginning)
    /// * `mkstream` - Create the stream if it doesn't exist
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use redis_oxide::{Client, ConnectionConfig};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ConnectionConfig::new("redis://localhost:6379");
    /// let client = Client::connect(config).await?;
    ///
    /// // Create a consumer group starting from the latest messages
    /// client.xgroup_create("events", "processors", "$", true).await?;
    /// println!("Consumer group created");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn xgroup_create(
        &self,
        stream: impl Into<String>,
        group: impl Into<String>,
        id: impl Into<String>,
        mkstream: bool,
    ) -> RedisResult<()> {
        let stream = stream.into();
        let mut cmd_args = vec![
            RespValue::from("CREATE"),
            RespValue::from(stream.clone()),
            RespValue::from(group.into()),
            RespValue::from(id.into()),
        ];

        if mkstream {
            cmd_args.push(RespValue::from("MKSTREAM"));
        }

        let result = match self.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.standalone_pool {
                    pool.execute_command("XGROUP".to_string(), cmd_args).await?
                } else {
                    return Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ));
                }
            }
            TopologyType::Cluster => {
                // For cluster, use the stream name to determine the slot
                let slot = calculate_slot(stream.as_bytes());
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

                if let Some(node_key) = node_key {
                    if let Some(pool) = self.get_cluster_pool(&node_key).await {
                        pool.execute_command("XGROUP".to_string(), cmd_args).await?
                    } else {
                        return Err(RedisError::Cluster(format!(
                            "Pool not found for node: {}",
                            node_key
                        )));
                    }
                } else {
                    return Err(RedisError::Cluster(format!(
                        "No node found for slot: {}",
                        slot
                    )));
                }
            }
        };

        // Expect "OK" response
        match result.as_string()?.as_str() {
            "OK" => Ok(()),
            other => Err(RedisError::Protocol(format!(
                "Unexpected XGROUP CREATE response: {}",
                other
            ))),
        }
    }

    /// Read from a consumer group using XREADGROUP
    ///
    /// # Arguments
    ///
    /// * `group` - The consumer group name
    /// * `consumer` - The consumer name
    /// * `streams` - Vector of (stream_name, id) pairs (">" for new messages)
    /// * `count` - Maximum number of entries per stream
    /// * `block` - Block timeout
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use redis_oxide::{Client, ConnectionConfig};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ConnectionConfig::new("redis://localhost:6379");
    /// let client = Client::connect(config).await?;
    ///
    /// // Read new messages from the group
    /// let streams = vec![("events".to_string(), ">".to_string())];
    /// let messages = client.xreadgroup(
    ///     "processors",
    ///     "worker-1",
    ///     streams,
    ///     Some(1),
    ///     Some(std::time::Duration::from_secs(1))
    /// ).await?;
    ///
    /// for (stream, entries) in messages {
    ///     for entry in entries {
    ///         println!("Processing {}: {:?}", entry.id, entry.fields);
    ///         // Acknowledge the message after processing
    ///         client.xack(&stream, "processors", vec![entry.id]).await?;
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn xreadgroup(
        &self,
        group: impl Into<String>,
        consumer: impl Into<String>,
        streams: Vec<(String, String)>,
        count: Option<u64>,
        block: Option<Duration>,
    ) -> RedisResult<std::collections::HashMap<String, Vec<crate::streams::StreamEntry>>> {
        let mut cmd_args = vec![
            RespValue::from("GROUP"),
            RespValue::from(group.into()),
            RespValue::from(consumer.into()),
        ];

        // Add COUNT option
        if let Some(count) = count {
            cmd_args.push(RespValue::from("COUNT"));
            cmd_args.push(RespValue::from(count.to_string()));
        }

        // Add BLOCK option
        if let Some(block) = block {
            cmd_args.push(RespValue::from("BLOCK"));
            cmd_args.push(RespValue::from(block.as_millis().to_string()));
        }

        // Add STREAMS keyword
        cmd_args.push(RespValue::from("STREAMS"));

        // Add stream names
        for (stream, _) in &streams {
            cmd_args.push(RespValue::from(stream.clone()));
        }

        // Add stream IDs
        for (_, id) in &streams {
            cmd_args.push(RespValue::from(id.clone()));
        }

        let result = match self.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.standalone_pool {
                    pool.execute_command("XREADGROUP".to_string(), cmd_args)
                        .await?
                } else {
                    return Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ));
                }
            }
            TopologyType::Cluster => {
                // For cluster, use any available node
                let pools = self.cluster_pools.read().await;
                if let Some((_, pool)) = pools.iter().next() {
                    pool.execute_command("XREADGROUP".to_string(), cmd_args)
                        .await?
                } else {
                    return Err(RedisError::Cluster(
                        "No cluster nodes available".to_string(),
                    ));
                }
            }
        };

        crate::streams::parse_xread_response(result)
    }

    /// Acknowledge messages using XACK
    ///
    /// # Arguments
    ///
    /// * `stream` - The stream name
    /// * `group` - The consumer group name
    /// * `ids` - Vector of message IDs to acknowledge
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use redis_oxide::{Client, ConnectionConfig};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = ConnectionConfig::new("redis://localhost:6379");
    /// let client = Client::connect(config).await?;
    ///
    /// // Acknowledge processed messages
    /// let acked = client.xack("events", "processors", vec![
    ///     "1234567890123-0".to_string(),
    ///     "1234567890124-0".to_string(),
    /// ]).await?;
    /// println!("Acknowledged {} messages", acked);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn xack(
        &self,
        stream: impl Into<String>,
        group: impl Into<String>,
        ids: Vec<String>,
    ) -> RedisResult<u64> {
        let stream = stream.into();
        let mut cmd_args = vec![
            RespValue::from(stream.clone()),
            RespValue::from(group.into()),
        ];

        for id in ids {
            cmd_args.push(RespValue::from(id));
        }

        let result = match self.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.standalone_pool {
                    pool.execute_command("XACK".to_string(), cmd_args).await?
                } else {
                    return Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ));
                }
            }
            TopologyType::Cluster => {
                // For cluster, use the stream name to determine the slot
                let slot = calculate_slot(stream.as_bytes());
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

                if let Some(node_key) = node_key {
                    if let Some(pool) = self.get_cluster_pool(&node_key).await {
                        pool.execute_command("XACK".to_string(), cmd_args).await?
                    } else {
                        return Err(RedisError::Cluster(format!(
                            "Pool not found for node: {}",
                            node_key
                        )));
                    }
                } else {
                    return Err(RedisError::Cluster(format!(
                        "No node found for slot: {}",
                        slot
                    )));
                }
            }
        };

        Ok(result.as_int()? as u64)
    }

    /// Get the topology type
    pub fn topology_type(&self) -> TopologyType {
        self.topology_type
    }
}

/// Pipeline executor implementation for Client
struct ClientPipelineExecutor {
    client: Client,
}

#[async_trait::async_trait]
impl PipelineExecutor for ClientPipelineExecutor {
    async fn execute_pipeline(
        &mut self,
        commands: Vec<Box<dyn PipelineCommand>>,
    ) -> RedisResult<Vec<RespValue>> {
        if commands.is_empty() {
            return Ok(Vec::new());
        }

        // For pipeline execution, we need to send all commands in one batch
        // We'll use the first command to determine the target node (for cluster mode)
        let first_command = &commands[0];
        let first_key = first_command.key();

        match self.client.topology_type {
            TopologyType::Standalone => {
                // For standalone, execute all commands on the single connection
                if let Some(pool) = &self.client.standalone_pool {
                    self.execute_pipeline_on_pool(pool, commands).await
                } else {
                    Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ))
                }
            }
            TopologyType::Cluster => {
                // For cluster mode, we need to group commands by their target slots
                // For simplicity in this initial implementation, we'll execute on the node
                // determined by the first command's key
                if let Some(key) = first_key {
                    let slot = calculate_slot(key.as_bytes());
                    let node_addr = self.get_node_for_slot(slot).await?;
                    let pool = self.get_or_create_pool(&node_addr).await?;
                    self.execute_pipeline_on_pool(&pool, commands).await
                } else {
                    // If no key, use any available node
                    let pools = self.client.cluster_pools.read().await;
                    if let Some((_, pool)) = pools.iter().next() {
                        self.execute_pipeline_on_pool(pool, commands).await
                    } else {
                        Err(RedisError::Cluster(
                            "No cluster nodes available".to_string(),
                        ))
                    }
                }
            }
        }
    }
}

impl ClientPipelineExecutor {
    /// Execute pipeline commands on a specific pool
    async fn execute_pipeline_on_pool(
        &self,
        pool: &Arc<Pool>,
        commands: Vec<Box<dyn PipelineCommand>>,
    ) -> RedisResult<Vec<RespValue>> {
        // Build the pipeline command array
        let mut pipeline_args = Vec::new();

        for command in commands {
            let mut cmd_args = vec![RespValue::from(command.name())];
            cmd_args.extend(command.args());
            pipeline_args.push(RespValue::Array(cmd_args));
        }

        // Execute all commands in the pipeline
        let mut results = Vec::new();
        for cmd_array in pipeline_args {
            if let RespValue::Array(args) = cmd_array {
                if let Some(RespValue::BulkString(cmd_name)) = args.first() {
                    let command = String::from_utf8_lossy(cmd_name).to_string();
                    let cmd_args = args.into_iter().skip(1).collect();

                    // For now, execute commands sequentially
                    // TODO: Implement true pipelining at the protocol level
                    let result = pool.execute_command(command, cmd_args).await?;
                    results.push(result);
                } else if let Some(RespValue::SimpleString(cmd_name)) = args.first() {
                    let command = cmd_name.clone();
                    let cmd_args = args.into_iter().skip(1).collect();

                    let result = pool.execute_command(command, cmd_args).await?;
                    results.push(result);
                }
            }
        }

        Ok(results)
    }

    /// Get the node address for a given slot (cluster mode)
    async fn get_node_for_slot(&self, slot: u16) -> RedisResult<String> {
        if let Some(topology) = &self.client.cluster_topology {
            if let Some((host, port)) = topology.get_node_for_slot(slot).await {
                Ok(format!("{}:{}", host, port))
            } else {
                Err(RedisError::Cluster(format!(
                    "No node found for slot {}",
                    slot
                )))
            }
        } else {
            Err(RedisError::Cluster(
                "No cluster topology available".to_string(),
            ))
        }
    }

    /// Get or create a pool for the given node address
    async fn get_or_create_pool(&self, node_addr: &str) -> RedisResult<Arc<Pool>> {
        let pools = self.client.cluster_pools.read().await;
        if let Some(pool) = pools.get(node_addr) {
            Ok(pool.clone())
        } else {
            drop(pools);

            // Create new pool for this node
            let mut pools = self.client.cluster_pools.write().await;

            // Double-check after acquiring write lock
            if let Some(pool) = pools.get(node_addr) {
                return Ok(pool.clone());
            }

            // Parse node address
            let parts: Vec<&str> = node_addr.split(':').collect();
            if parts.len() != 2 {
                return Err(RedisError::Config(format!(
                    "Invalid node address: {}",
                    node_addr
                )));
            }

            let host = parts[0];
            let port: u16 = parts[1].parse().map_err(|_| {
                RedisError::Config(format!("Invalid port in address: {}", node_addr))
            })?;

            // Create config for this node
            let node_config = self.client.config.clone();

            let pool = Arc::new(Pool::new(node_config, host.to_string(), port).await?);
            pools.insert(node_addr.to_string(), pool.clone());

            Ok(pool)
        }
    }
}

/// Transaction executor implementation for Client
struct ClientTransactionExecutor {
    client: Client,
}

#[async_trait::async_trait]
impl TransactionExecutor for ClientTransactionExecutor {
    async fn multi(&mut self) -> RedisResult<()> {
        // Execute MULTI command
        match self.client.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.client.standalone_pool {
                    let _result = pool.execute_command("MULTI".to_string(), vec![]).await?;
                    Ok(())
                } else {
                    Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ))
                }
            }
            TopologyType::Cluster => {
                // For cluster, use any available node for transaction
                let pools = self.client.cluster_pools.read().await;
                if let Some((_, pool)) = pools.iter().next() {
                    let _result = pool.execute_command("MULTI".to_string(), vec![]).await?;
                    Ok(())
                } else {
                    Err(RedisError::Cluster(
                        "No cluster nodes available".to_string(),
                    ))
                }
            }
        }
    }

    async fn queue_command(&mut self, command: Box<dyn TransactionCommand>) -> RedisResult<()> {
        // Execute the command (it will be queued by Redis after MULTI)
        let cmd_name = command.name().to_string();
        let cmd_args = command.args();

        match self.client.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.client.standalone_pool {
                    let _result = pool.execute_command(cmd_name, cmd_args).await?;
                    Ok(())
                } else {
                    Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ))
                }
            }
            TopologyType::Cluster => {
                // For cluster, use the node determined by the command's key
                if let Some(key) = command.key() {
                    let slot = calculate_slot(key.as_bytes());
                    let node_addr = self.get_node_for_slot(slot).await?;
                    let pool = self.get_or_create_pool(&node_addr).await?;
                    let _result = pool.execute_command(cmd_name, cmd_args).await?;
                    Ok(())
                } else {
                    // If no key, use any available node
                    let pools = self.client.cluster_pools.read().await;
                    if let Some((_, pool)) = pools.iter().next() {
                        let _result = pool.execute_command(cmd_name, cmd_args).await?;
                        Ok(())
                    } else {
                        Err(RedisError::Cluster(
                            "No cluster nodes available".to_string(),
                        ))
                    }
                }
            }
        }
    }

    async fn exec(&mut self) -> RedisResult<Vec<RespValue>> {
        // Execute EXEC command
        match self.client.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.client.standalone_pool {
                    let result = pool.execute_command("EXEC".to_string(), vec![]).await?;
                    match result {
                        RespValue::Array(results) => Ok(results),
                        RespValue::Null => Ok(vec![]), // Transaction was discarded (watched key changed)
                        _ => Err(RedisError::Type(format!(
                            "Unexpected EXEC response: {:?}",
                            result
                        ))),
                    }
                } else {
                    Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ))
                }
            }
            TopologyType::Cluster => {
                let pools = self.client.cluster_pools.read().await;
                if let Some((_, pool)) = pools.iter().next() {
                    let result = pool.execute_command("EXEC".to_string(), vec![]).await?;
                    match result {
                        RespValue::Array(results) => Ok(results),
                        RespValue::Null => Ok(vec![]), // Transaction was discarded (watched key changed)
                        _ => Err(RedisError::Type(format!(
                            "Unexpected EXEC response: {:?}",
                            result
                        ))),
                    }
                } else {
                    Err(RedisError::Cluster(
                        "No cluster nodes available".to_string(),
                    ))
                }
            }
        }
    }

    async fn discard(&mut self) -> RedisResult<()> {
        // Execute DISCARD command
        match self.client.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.client.standalone_pool {
                    let _result = pool.execute_command("DISCARD".to_string(), vec![]).await?;
                    Ok(())
                } else {
                    Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ))
                }
            }
            TopologyType::Cluster => {
                let pools = self.client.cluster_pools.read().await;
                if let Some((_, pool)) = pools.iter().next() {
                    let _result = pool.execute_command("DISCARD".to_string(), vec![]).await?;
                    Ok(())
                } else {
                    Err(RedisError::Cluster(
                        "No cluster nodes available".to_string(),
                    ))
                }
            }
        }
    }

    async fn watch(&mut self, keys: Vec<String>) -> RedisResult<()> {
        // Execute WATCH command
        let mut args = vec![];
        for key in keys {
            args.push(RespValue::from(key));
        }

        match self.client.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.client.standalone_pool {
                    let _result = pool.execute_command("WATCH".to_string(), args).await?;
                    Ok(())
                } else {
                    Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ))
                }
            }
            TopologyType::Cluster => {
                let pools = self.client.cluster_pools.read().await;
                if let Some((_, pool)) = pools.iter().next() {
                    let _result = pool.execute_command("WATCH".to_string(), args).await?;
                    Ok(())
                } else {
                    Err(RedisError::Cluster(
                        "No cluster nodes available".to_string(),
                    ))
                }
            }
        }
    }

    async fn unwatch(&mut self) -> RedisResult<()> {
        // Execute UNWATCH command
        match self.client.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.client.standalone_pool {
                    let _result = pool.execute_command("UNWATCH".to_string(), vec![]).await?;
                    Ok(())
                } else {
                    Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ))
                }
            }
            TopologyType::Cluster => {
                let pools = self.client.cluster_pools.read().await;
                if let Some((_, pool)) = pools.iter().next() {
                    let _result = pool.execute_command("UNWATCH".to_string(), vec![]).await?;
                    Ok(())
                } else {
                    Err(RedisError::Cluster(
                        "No cluster nodes available".to_string(),
                    ))
                }
            }
        }
    }
}

impl ClientTransactionExecutor {
    /// Get the node address for a given slot (cluster mode)
    async fn get_node_for_slot(&self, slot: u16) -> RedisResult<String> {
        if let Some(topology) = &self.client.cluster_topology {
            if let Some((host, port)) = topology.get_node_for_slot(slot).await {
                Ok(format!("{}:{}", host, port))
            } else {
                Err(RedisError::Cluster(format!(
                    "No node found for slot {}",
                    slot
                )))
            }
        } else {
            Err(RedisError::Cluster(
                "No cluster topology available".to_string(),
            ))
        }
    }

    /// Get or create a pool for the given node address
    async fn get_or_create_pool(&self, node_addr: &str) -> RedisResult<Arc<Pool>> {
        let pools = self.client.cluster_pools.read().await;
        if let Some(pool) = pools.get(node_addr) {
            Ok(pool.clone())
        } else {
            drop(pools);

            // Create new pool for this node
            let mut pools = self.client.cluster_pools.write().await;

            // Double-check after acquiring write lock
            if let Some(pool) = pools.get(node_addr) {
                return Ok(pool.clone());
            }

            // Parse node address
            let parts: Vec<&str> = node_addr.split(':').collect();
            if parts.len() != 2 {
                return Err(RedisError::Config(format!(
                    "Invalid node address: {}",
                    node_addr
                )));
            }

            let host = parts[0];
            let port: u16 = parts[1].parse().map_err(|_| {
                RedisError::Config(format!("Invalid port in address: {}", node_addr))
            })?;

            // Create config for this node
            let node_config = self.client.config.clone();

            let pool = Arc::new(Pool::new(node_config, host.to_string(), port).await?);
            pools.insert(node_addr.to_string(), pool.clone());

            Ok(pool)
        }
    }
}

/// Pub/Sub connection implementation for Client
struct ClientPubSubConnection {
    client: Client,
}

#[async_trait::async_trait]
impl PubSubConnection for ClientPubSubConnection {
    async fn subscribe(&mut self, channels: Vec<String>) -> RedisResult<()> {
        let mut args = vec![];
        for channel in channels {
            args.push(RespValue::from(channel));
        }

        match self.client.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.client.standalone_pool {
                    let _result = pool.execute_command("SUBSCRIBE".to_string(), args).await?;
                    Ok(())
                } else {
                    Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ))
                }
            }
            TopologyType::Cluster => {
                let pools = self.client.cluster_pools.read().await;
                if let Some((_, pool)) = pools.iter().next() {
                    let _result = pool.execute_command("SUBSCRIBE".to_string(), args).await?;
                    Ok(())
                } else {
                    Err(RedisError::Cluster(
                        "No cluster nodes available".to_string(),
                    ))
                }
            }
        }
    }

    async fn unsubscribe(&mut self, channels: Vec<String>) -> RedisResult<()> {
        let mut args = vec![];
        for channel in channels {
            args.push(RespValue::from(channel));
        }

        match self.client.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.client.standalone_pool {
                    let _result = pool
                        .execute_command("UNSUBSCRIBE".to_string(), args)
                        .await?;
                    Ok(())
                } else {
                    Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ))
                }
            }
            TopologyType::Cluster => {
                let pools = self.client.cluster_pools.read().await;
                if let Some((_, pool)) = pools.iter().next() {
                    let _result = pool
                        .execute_command("UNSUBSCRIBE".to_string(), args)
                        .await?;
                    Ok(())
                } else {
                    Err(RedisError::Cluster(
                        "No cluster nodes available".to_string(),
                    ))
                }
            }
        }
    }

    async fn psubscribe(&mut self, patterns: Vec<String>) -> RedisResult<()> {
        let mut args = vec![];
        for pattern in patterns {
            args.push(RespValue::from(pattern));
        }

        match self.client.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.client.standalone_pool {
                    let _result = pool.execute_command("PSUBSCRIBE".to_string(), args).await?;
                    Ok(())
                } else {
                    Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ))
                }
            }
            TopologyType::Cluster => {
                let pools = self.client.cluster_pools.read().await;
                if let Some((_, pool)) = pools.iter().next() {
                    let _result = pool.execute_command("PSUBSCRIBE".to_string(), args).await?;
                    Ok(())
                } else {
                    Err(RedisError::Cluster(
                        "No cluster nodes available".to_string(),
                    ))
                }
            }
        }
    }

    async fn punsubscribe(&mut self, patterns: Vec<String>) -> RedisResult<()> {
        let mut args = vec![];
        for pattern in patterns {
            args.push(RespValue::from(pattern));
        }

        match self.client.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.client.standalone_pool {
                    let _result = pool
                        .execute_command("PUNSUBSCRIBE".to_string(), args)
                        .await?;
                    Ok(())
                } else {
                    Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ))
                }
            }
            TopologyType::Cluster => {
                let pools = self.client.cluster_pools.read().await;
                if let Some((_, pool)) = pools.iter().next() {
                    let _result = pool
                        .execute_command("PUNSUBSCRIBE".to_string(), args)
                        .await?;
                    Ok(())
                } else {
                    Err(RedisError::Cluster(
                        "No cluster nodes available".to_string(),
                    ))
                }
            }
        }
    }

    async fn listen(
        &mut self,
        message_tx: tokio::sync::mpsc::UnboundedSender<crate::pubsub::PubSubMessage>,
    ) -> RedisResult<()> {
        // This is a simplified implementation
        // In a real implementation, this would maintain a persistent connection
        // and continuously listen for pub/sub messages

        // For now, we'll just return Ok to satisfy the trait
        // A full implementation would require a dedicated connection for pub/sub
        // that stays open and continuously reads messages

        // TODO: Implement proper pub/sub message listening
        // This would involve:
        // 1. Creating a dedicated connection for pub/sub
        // 2. Continuously reading RESP messages
        // 3. Parsing pub/sub messages and sending them through message_tx

        drop(message_tx); // Avoid unused variable warning
        Ok(())
    }

    async fn publish(&mut self, channel: String, message: String) -> RedisResult<i64> {
        let args = vec![RespValue::from(channel), RespValue::from(message)];

        match self.client.topology_type {
            TopologyType::Standalone => {
                if let Some(pool) = &self.client.standalone_pool {
                    let result = pool.execute_command("PUBLISH".to_string(), args).await?;
                    result.as_int()
                } else {
                    Err(RedisError::Connection(
                        "No standalone pool available".to_string(),
                    ))
                }
            }
            TopologyType::Cluster => {
                let pools = self.client.cluster_pools.read().await;
                if let Some((_, pool)) = pools.iter().next() {
                    let result = pool.execute_command("PUBLISH".to_string(), args).await?;
                    result.as_int()
                } else {
                    Err(RedisError::Cluster(
                        "No cluster nodes available".to_string(),
                    ))
                }
            }
        }
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
