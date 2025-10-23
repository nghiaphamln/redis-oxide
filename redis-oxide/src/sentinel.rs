//! Redis Sentinel support for high availability
//!
//! Redis Sentinel provides high availability for Redis. It monitors Redis master
//! and slave instances, performs automatic failover when a master is not working
//! as expected, and acts as a configuration provider for clients.
//!
//! # Examples
//!
//! ## Basic Sentinel Configuration
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig, SentinelConfig};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let sentinel_config = SentinelConfig::new("mymaster")
//!     .add_sentinel("127.0.0.1:26379")
//!     .add_sentinel("127.0.0.1:26380")
//!     .add_sentinel("127.0.0.1:26381")
//!     .with_password("sentinel_password");
//!
//! let config = ConnectionConfig::new_with_sentinel(sentinel_config);
//! let client = Client::connect(config).await?;
//!
//! // Client automatically connects to current master
//! client.set("key", "value").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Handling Failover
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig, SentinelConfig};
//! use std::time::Duration;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let sentinel_config = SentinelConfig::new("mymaster")
//!     .add_sentinel("127.0.0.1:26379")
//!     .with_failover_timeout(Duration::from_secs(30));
//!
//! let config = ConnectionConfig::new_with_sentinel(sentinel_config);
//! let client = Client::connect(config).await?;
//!
//! // Client automatically handles master failover
//! loop {
//!     match client.get("key").await {
//!         Ok(_) => {
//!             println!("Connected to master");
//!             break;
//!         }
//!         Err(e) => {
//!             println!("Connection failed: {}, retrying...", e);
//!             tokio::time::sleep(Duration::from_secs(1)).await;
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use crate::connection::RedisConnection;
use crate::core::{
    config::ConnectionConfig,
    error::{RedisError, RedisResult},
    value::RespValue,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

/// Configuration for Redis Sentinel
#[derive(Debug, Clone)]
pub struct SentinelConfig {
    /// Master name to monitor
    pub master_name: String,
    /// List of sentinel endpoints
    pub sentinels: Vec<SentinelEndpoint>,
    /// Password for sentinel authentication
    pub password: Option<String>,
    /// Timeout for failover operations
    pub failover_timeout: Duration,
    /// Interval for checking master status
    pub check_interval: Duration,
    /// Maximum number of failover retries
    pub max_retries: usize,
}

/// Sentinel endpoint configuration
#[derive(Debug, Clone)]
pub struct SentinelEndpoint {
    /// Sentinel host
    pub host: String,
    /// Sentinel port
    pub port: u16,
}

impl SentinelEndpoint {
    /// Create a new sentinel endpoint
    #[must_use]
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }

    /// Parse from address string (host:port)
    ///
    /// # Errors
    ///
    /// Returns an error if the address format is invalid.
    pub fn from_address(addr: &str) -> RedisResult<Self> {
        let parts: Vec<&str> = addr.split(':').collect();
        if parts.len() != 2 {
            return Err(RedisError::Config(format!(
                "Invalid sentinel address: {}",
                addr
            )));
        }

        let host = parts[0].to_string();
        let port = parts[1].parse::<u16>().map_err(|_| {
            RedisError::Config(format!("Invalid port in sentinel address: {}", addr))
        })?;

        Ok(Self::new(host, port))
    }

    /// Get the address string
    #[must_use]
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

impl SentinelConfig {
    /// Create a new sentinel configuration
    #[must_use]
    pub fn new(master_name: impl Into<String>) -> Self {
        Self {
            master_name: master_name.into(),
            sentinels: Vec::new(),
            password: None,
            failover_timeout: Duration::from_secs(30),
            check_interval: Duration::from_secs(5),
            max_retries: 3,
        }
    }

    /// Add a sentinel endpoint
    #[must_use]
    pub fn add_sentinel(mut self, addr: impl AsRef<str>) -> Self {
        if let Ok(endpoint) = SentinelEndpoint::from_address(addr.as_ref()) {
            self.sentinels.push(endpoint);
        }
        self
    }

    /// Set sentinel password
    #[must_use]
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Set failover timeout
    #[must_use]
    pub const fn with_failover_timeout(mut self, timeout: Duration) -> Self {
        self.failover_timeout = timeout;
        self
    }

    /// Set check interval
    #[must_use]
    pub const fn with_check_interval(mut self, interval: Duration) -> Self {
        self.check_interval = interval;
        self
    }

    /// Set maximum retries
    #[must_use]
    pub const fn with_max_retries(mut self, retries: usize) -> Self {
        self.max_retries = retries;
        self
    }
}

/// Information about a Redis master
#[derive(Debug, Clone)]
pub struct MasterInfo {
    /// Master name
    pub name: String,
    /// Master host
    pub host: String,
    /// Master port
    pub port: u16,
    /// Master status flags
    pub flags: Vec<String>,
    /// Number of connected slaves
    pub num_slaves: u32,
    /// Number of other sentinels
    pub num_other_sentinels: u32,
    /// Quorum for failover
    pub quorum: u32,
    /// Failover timeout
    pub failover_timeout: Duration,
    /// Parallel syncs
    pub parallel_syncs: u32,
}

impl MasterInfo {
    /// Check if master is down
    #[must_use]
    pub fn is_down(&self) -> bool {
        self.flags.contains(&"s_down".to_string()) || self.flags.contains(&"o_down".to_string())
    }

    /// Check if failover is in progress
    #[must_use]
    pub fn is_failover_in_progress(&self) -> bool {
        self.flags.contains(&"failover_in_progress".to_string())
    }

    /// Get master address
    #[must_use]
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Redis Sentinel client for high availability
pub struct SentinelClient {
    config: SentinelConfig,
    sentinels: Arc<RwLock<Vec<Arc<Mutex<RedisConnection>>>>>,
    current_master: Arc<RwLock<Option<MasterInfo>>>,
    last_check: Arc<Mutex<Instant>>,
}

impl SentinelClient {
    /// Create a new sentinel client
    ///
    /// # Errors
    ///
    /// Returns an error if no sentinels are configured.
    pub async fn new(config: SentinelConfig) -> RedisResult<Self> {
        if config.sentinels.is_empty() {
            return Err(RedisError::Config("No sentinels configured".to_string()));
        }

        let client = Self {
            config,
            sentinels: Arc::new(RwLock::new(Vec::new())),
            current_master: Arc::new(RwLock::new(None)),
            last_check: Arc::new(Mutex::new(Instant::now())),
        };

        // Initialize sentinel connections
        client.initialize_sentinels().await?;

        // Discover current master
        client.discover_master().await?;

        Ok(client)
    }

    /// Get current master information
    ///
    /// # Errors
    ///
    /// Returns an error if no master is available.
    pub async fn get_master(&self) -> RedisResult<MasterInfo> {
        // Check if we need to refresh master info
        {
            let last_check = self.last_check.lock().await;
            if last_check.elapsed() < self.config.check_interval {
                if let Some(master) = self.current_master.read().await.clone() {
                    return Ok(master);
                }
            }
        }

        // Refresh master info
        self.discover_master().await?;

        self.current_master
            .read()
            .await
            .clone()
            .ok_or_else(|| RedisError::Sentinel("No master available".to_string()))
    }

    /// Create a connection to the current master
    ///
    /// # Errors
    ///
    /// Returns an error if master connection fails.
    pub async fn connect_to_master(&self) -> RedisResult<RedisConnection> {
        let master = self.get_master().await?;

        let master_config =
            ConnectionConfig::new(&format!("redis://{}:{}", master.host, master.port));

        RedisConnection::connect(&master.host, master.port, master_config).await
    }

    /// Monitor for master changes and failovers
    pub async fn monitor(&self) -> RedisResult<()> {
        let mut interval = tokio::time::interval(self.config.check_interval);

        loop {
            interval.tick().await;

            if let Err(e) = self.check_master_status().await {
                warn!("Failed to check master status: {}", e);
            }
        }
    }

    async fn initialize_sentinels(&self) -> RedisResult<()> {
        let mut sentinels = self.sentinels.write().await;

        for endpoint in &self.config.sentinels {
            match self.connect_to_sentinel(endpoint).await {
                Ok(conn) => {
                    sentinels.push(Arc::new(Mutex::new(conn)));
                    info!("Connected to sentinel: {}", endpoint.address());
                }
                Err(e) => {
                    warn!(
                        "Failed to connect to sentinel {}: {}",
                        endpoint.address(),
                        e
                    );
                }
            }
        }

        if sentinels.is_empty() {
            return Err(RedisError::Sentinel("No sentinels available".to_string()));
        }

        Ok(())
    }

    async fn connect_to_sentinel(
        &self,
        endpoint: &SentinelEndpoint,
    ) -> RedisResult<RedisConnection> {
        let sentinel_config =
            ConnectionConfig::new(&format!("redis://{}:{}", endpoint.host, endpoint.port));

        let mut conn =
            RedisConnection::connect(&endpoint.host, endpoint.port, sentinel_config).await?;

        // Authenticate if password is provided
        if let Some(password) = &self.config.password {
            let auth_cmd = RespValue::Array(vec![
                RespValue::BulkString(bytes::Bytes::from("AUTH")),
                RespValue::BulkString(bytes::Bytes::from(password.clone())),
            ]);

            conn.send_command(&auth_cmd).await?;
            let _response = conn.read_response().await?;
        }

        Ok(conn)
    }

    async fn discover_master(&self) -> RedisResult<()> {
        let sentinels = self.sentinels.read().await;

        for sentinel in sentinels.iter() {
            match self.query_master_info(sentinel).await {
                Ok(master_info) => {
                    info!("Discovered master: {}", master_info.address());
                    *self.current_master.write().await = Some(master_info);
                    *self.last_check.lock().await = Instant::now();
                    return Ok(());
                }
                Err(e) => {
                    debug!("Failed to query master from sentinel: {}", e);
                }
            }
        }

        Err(RedisError::Sentinel(
            "Failed to discover master from any sentinel".to_string(),
        ))
    }

    async fn query_master_info(
        &self,
        sentinel: &Arc<Mutex<RedisConnection>>,
    ) -> RedisResult<MasterInfo> {
        let mut conn = sentinel.lock().await;

        // Send SENTINEL masters command
        let cmd = RespValue::Array(vec![
            RespValue::BulkString(bytes::Bytes::from("SENTINEL")),
            RespValue::BulkString(bytes::Bytes::from("masters")),
        ]);

        conn.send_command(&cmd).await?;
        let response = conn.read_response().await?;

        self.parse_master_info(response)
    }

    fn parse_master_info(&self, response: RespValue) -> RedisResult<MasterInfo> {
        match response {
            RespValue::Array(masters) => {
                for master in masters {
                    if let RespValue::Array(master_data) = master {
                        let master_info = self.parse_single_master(master_data)?;
                        if master_info.name == self.config.master_name {
                            return Ok(master_info);
                        }
                    }
                }
                Err(RedisError::Sentinel(format!(
                    "Master '{}' not found",
                    self.config.master_name
                )))
            }
            _ => Err(RedisError::Sentinel("Invalid masters response".to_string())),
        }
    }

    fn parse_single_master(&self, master_data: Vec<RespValue>) -> RedisResult<MasterInfo> {
        let mut info = HashMap::new();

        // Parse key-value pairs
        for chunk in master_data.chunks(2) {
            if chunk.len() == 2 {
                let key = chunk[0].as_string()?;
                let value = chunk[1].as_string()?;
                info.insert(key, value);
            }
        }

        let name = info
            .get("name")
            .ok_or_else(|| RedisError::Sentinel("Missing master name".to_string()))?
            .clone();

        let host = info
            .get("ip")
            .ok_or_else(|| RedisError::Sentinel("Missing master IP".to_string()))?
            .clone();

        let port = info
            .get("port")
            .ok_or_else(|| RedisError::Sentinel("Missing master port".to_string()))?
            .parse::<u16>()
            .map_err(|_| RedisError::Sentinel("Invalid master port".to_string()))?;

        let flags = info
            .get("flags")
            .map(|f| f.split(',').map(String::from).collect())
            .unwrap_or_default();

        let num_slaves = info
            .get("num-slaves")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let num_other_sentinels = info
            .get("num-other-sentinels")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let quorum = info.get("quorum").and_then(|s| s.parse().ok()).unwrap_or(1);

        let failover_timeout = info
            .get("failover-timeout")
            .and_then(|s| s.parse().ok())
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_secs(60));

        let parallel_syncs = info
            .get("parallel-syncs")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        Ok(MasterInfo {
            name,
            host,
            port,
            flags,
            num_slaves,
            num_other_sentinels,
            quorum,
            failover_timeout,
            parallel_syncs,
        })
    }

    async fn check_master_status(&self) -> RedisResult<()> {
        let current_master = self.current_master.read().await.clone();

        if let Some(master) = current_master {
            // Try to connect to current master
            match self.test_master_connection(&master).await {
                Ok(_) => {
                    debug!("Master {} is healthy", master.address());
                }
                Err(_) => {
                    warn!(
                        "Master {} is not responding, discovering new master",
                        master.address()
                    );
                    self.discover_master().await?;
                }
            }
        } else {
            // No current master, try to discover
            self.discover_master().await?;
        }

        Ok(())
    }

    async fn test_master_connection(&self, master: &MasterInfo) -> RedisResult<()> {
        let master_config =
            ConnectionConfig::new(&format!("redis://{}:{}", master.host, master.port));
        let mut conn = RedisConnection::connect(&master.host, master.port, master_config).await?;

        // Send PING command
        let ping_cmd = RespValue::Array(vec![RespValue::BulkString(bytes::Bytes::from("PING"))]);

        conn.send_command(&ping_cmd).await?;
        let response = conn.read_response().await?;

        match response {
            RespValue::SimpleString(s) if s == "PONG" => Ok(()),
            _ => Err(RedisError::Connection(
                "Master did not respond to PING".to_string(),
            )),
        }
    }
}

/// Extension trait for ConnectionConfig to support Sentinel
impl ConnectionConfig {
    /// Create a new connection config with Sentinel
    #[must_use]
    pub fn new_with_sentinel(sentinel_config: SentinelConfig) -> Self {
        let mut config = Self::new("");
        config.sentinel = Some(sentinel_config);
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sentinel_endpoint_creation() {
        let endpoint = SentinelEndpoint::new("localhost", 26379);
        assert_eq!(endpoint.host, "localhost");
        assert_eq!(endpoint.port, 26379);
        assert_eq!(endpoint.address(), "localhost:26379");
    }

    #[test]
    fn test_sentinel_endpoint_from_address() {
        let endpoint = SentinelEndpoint::from_address("127.0.0.1:26379").unwrap();
        assert_eq!(endpoint.host, "127.0.0.1");
        assert_eq!(endpoint.port, 26379);

        let invalid = SentinelEndpoint::from_address("invalid");
        assert!(invalid.is_err());
    }

    #[test]
    fn test_sentinel_config_builder() {
        let config = SentinelConfig::new("mymaster")
            .add_sentinel("127.0.0.1:26379")
            .add_sentinel("127.0.0.1:26380")
            .with_password("secret")
            .with_failover_timeout(Duration::from_secs(60))
            .with_max_retries(5);

        assert_eq!(config.master_name, "mymaster");
        assert_eq!(config.sentinels.len(), 2);
        assert_eq!(config.password, Some("secret".to_string()));
        assert_eq!(config.failover_timeout, Duration::from_secs(60));
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_master_info_status() {
        let mut master = MasterInfo {
            name: "mymaster".to_string(),
            host: "127.0.0.1".to_string(),
            port: 6379,
            flags: vec!["master".to_string()],
            num_slaves: 2,
            num_other_sentinels: 2,
            quorum: 2,
            failover_timeout: Duration::from_secs(60),
            parallel_syncs: 1,
        };

        assert!(!master.is_down());
        assert!(!master.is_failover_in_progress());

        master.flags.push("s_down".to_string());
        assert!(master.is_down());

        master.flags.clear();
        master.flags.push("failover_in_progress".to_string());
        assert!(master.is_failover_in_progress());
    }

    #[test]
    fn test_connection_config_with_sentinel() {
        let sentinel_config = SentinelConfig::new("mymaster").add_sentinel("127.0.0.1:26379");

        let config = ConnectionConfig::new_with_sentinel(sentinel_config);
        assert!(config.sentinel.is_some());
        assert_eq!(config.sentinel.as_ref().unwrap().master_name, "mymaster");
    }
}
