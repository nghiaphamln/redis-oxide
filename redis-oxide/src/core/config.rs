//! Configuration types for Redis connections

use std::time::Duration;

/// Protocol version preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProtocolVersion {
    /// RESP2 (Redis Serialization Protocol version 2) - Default
    #[default]
    Resp2,
    /// RESP3 (Redis Serialization Protocol version 3) - Redis 6.0+
    Resp3,
}

/// Strategy for connection pooling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolStrategy {
    /// Single multiplexed connection shared across tasks
    Multiplexed,
    /// Connection pool with multiple connections
    Pool,
}

/// Configuration for connection pooling
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Pooling strategy to use
    pub strategy: PoolStrategy,
    /// Maximum number of connections in pool (only for Pool strategy)
    pub max_size: usize,
    /// Minimum number of connections to maintain (only for Pool strategy)
    pub min_idle: usize,
    /// Timeout for acquiring a connection from pool
    pub connection_timeout: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            strategy: PoolStrategy::Multiplexed,
            max_size: 10,
            min_idle: 2,
            connection_timeout: Duration::from_secs(5),
        }
    }
}

/// Topology detection mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopologyMode {
    /// Automatically detect topology (Standalone or Cluster)
    Auto,
    /// Force standalone mode
    Standalone,
    /// Force cluster mode
    Cluster,
}

/// Configuration for Redis connection
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// Connection string (e.g., `<redis://localhost:6379>` or `<redis://host1:6379,host2:6379>`)
    pub connection_string: String,

    /// Optional password for authentication
    pub password: Option<String>,

    /// Database number (only for standalone mode)
    pub database: u8,

    /// Connection timeout
    pub connect_timeout: Duration,

    /// Read/write operation timeout
    pub operation_timeout: Duration,

    /// Enable TCP keepalive
    pub tcp_keepalive: Option<Duration>,

    /// Topology detection mode
    pub topology_mode: TopologyMode,

    /// Pool configuration
    pub pool: PoolConfig,

    /// Maximum number of retries for cluster redirects
    pub max_redirects: usize,

    /// Preferred protocol version
    pub protocol_version: ProtocolVersion,

    /// Sentinel configuration for high availability
    pub sentinel: Option<crate::sentinel::SentinelConfig>,

    /// Reconnection settings
    pub reconnect: ReconnectConfig,
}

/// Configuration for reconnection behavior
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// Enable automatic reconnection
    pub enabled: bool,

    /// Initial delay before first reconnect attempt
    pub initial_delay: Duration,

    /// Maximum delay between reconnect attempts
    pub max_delay: Duration,

    /// Backoff multiplier
    pub backoff_multiplier: f64,

    /// Maximum number of reconnect attempts (None = infinite)
    pub max_attempts: Option<usize>,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            max_attempts: None,
        }
    }
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            connection_string: "redis://localhost:6379".to_string(),
            password: None,
            database: 0,
            connect_timeout: Duration::from_secs(5),
            operation_timeout: Duration::from_secs(30),
            tcp_keepalive: Some(Duration::from_secs(60)),
            topology_mode: TopologyMode::Auto,
            pool: PoolConfig::default(),
            max_redirects: 3,
            protocol_version: ProtocolVersion::default(),
            sentinel: None,
            reconnect: ReconnectConfig::default(),
        }
    }
}

impl ConnectionConfig {
    /// Create a new configuration with the given connection string
    pub fn new(connection_string: impl Into<String>) -> Self {
        Self {
            connection_string: connection_string.into(),
            ..Default::default()
        }
    }

    /// Set the password for authentication
    #[must_use]
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Set the database number
    #[must_use]
    pub const fn with_database(mut self, database: u8) -> Self {
        self.database = database;
        self
    }

    /// Set the connection timeout
    #[must_use]
    pub const fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set the operation timeout
    #[must_use]
    pub const fn with_operation_timeout(mut self, timeout: Duration) -> Self {
        self.operation_timeout = timeout;
        self
    }

    /// Set the topology mode
    #[must_use]
    pub const fn with_topology_mode(mut self, mode: TopologyMode) -> Self {
        self.topology_mode = mode;
        self
    }

    /// Set the pool configuration
    #[must_use]
    pub const fn with_pool_config(mut self, pool: PoolConfig) -> Self {
        self.pool = pool;
        self
    }

    /// Set the maximum number of redirects
    #[must_use]
    pub const fn with_max_redirects(mut self, max: usize) -> Self {
        self.max_redirects = max;
        self
    }

    /// Set the preferred protocol version
    #[must_use]
    pub const fn with_protocol_version(mut self, version: ProtocolVersion) -> Self {
        self.protocol_version = version;
        self
    }

    /// Parse connection endpoints from connection string
    #[must_use]
    pub fn parse_endpoints(&self) -> Vec<(String, u16)> {
        let conn_str = self.connection_string.trim();

        // Strip redis:// prefix if present
        let addr_part = conn_str
            .strip_prefix("redis://")
            .unwrap_or(conn_str)
            .strip_prefix("rediss://")
            .unwrap_or_else(|| conn_str.strip_prefix("redis://").unwrap_or(conn_str));

        // Split by comma for multiple endpoints
        addr_part
            .split(',')
            .filter_map(|endpoint| {
                let endpoint = endpoint.trim();
                if endpoint.is_empty() {
                    return None;
                }

                // Parse host:port
                if let Some((host, port_str)) = endpoint.rsplit_once(':') {
                    if let Ok(port) = port_str.parse::<u16>() {
                        return Some((host.to_string(), port));
                    }
                }

                // Default port 6379 if not specified
                Some((endpoint.to_string(), 6379))
            })
            .collect()
    }
}
