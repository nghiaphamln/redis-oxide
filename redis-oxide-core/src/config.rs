//! Configuration types for Redis connections

use std::time::Duration;

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
    /// Connection string (e.g., "redis://localhost:6379" or "redis://host1:6379,host2:6379")
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
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Set the database number
    pub fn with_database(mut self, database: u8) -> Self {
        self.database = database;
        self
    }

    /// Set the connection timeout
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set the operation timeout
    pub fn with_operation_timeout(mut self, timeout: Duration) -> Self {
        self.operation_timeout = timeout;
        self
    }

    /// Set the topology mode
    pub fn with_topology_mode(mut self, mode: TopologyMode) -> Self {
        self.topology_mode = mode;
        self
    }

    /// Set the pool configuration
    pub fn with_pool_config(mut self, pool: PoolConfig) -> Self {
        self.pool = pool;
        self
    }

    /// Set the maximum number of redirects
    pub fn with_max_redirects(mut self, max: usize) -> Self {
        self.max_redirects = max;
        self
    }

    /// Parse connection endpoints from connection string
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_endpoint() {
        let config = ConnectionConfig::new("redis://localhost:6379");
        let endpoints = config.parse_endpoints();
        assert_eq!(endpoints, vec![("localhost".to_string(), 6379)]);
    }

    #[test]
    fn test_parse_multiple_endpoints() {
        let config = ConnectionConfig::new("redis://host1:6379,host2:6380,host3:6381");
        let endpoints = config.parse_endpoints();
        assert_eq!(
            endpoints,
            vec![
                ("host1".to_string(), 6379),
                ("host2".to_string(), 6380),
                ("host3".to_string(), 6381),
            ]
        );
    }

    #[test]
    fn test_parse_endpoint_default_port() {
        let config = ConnectionConfig::new("redis://localhost");
        let endpoints = config.parse_endpoints();
        assert_eq!(endpoints, vec![("localhost".to_string(), 6379)]);
    }

    #[test]
    fn test_builder_pattern() {
        let config = ConnectionConfig::new("redis://localhost:6379")
            .with_password("secret")
            .with_database(5)
            .with_max_redirects(5);

        assert_eq!(config.password, Some("secret".to_string()));
        assert_eq!(config.database, 5);
        assert_eq!(config.max_redirects, 5);
    }
}
