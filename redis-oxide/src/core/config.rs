//! Configuration types for Redis connections.

use crate::core::error::{RedisError, RedisResult};
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
    /// Maximum number of connections in a connection pool.
    pub max_size: usize,
    /// Minimum number of idle connections to create for the Pool strategy.
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

    /// Validate configuration before opening a connection.
    ///
    /// # Errors
    ///
    /// Returns an error when a timeout, pool, reconnect, or endpoint setting
    /// cannot be used safely.
    pub fn validate(&self) -> RedisResult<()> {
        if self.connect_timeout.is_zero() {
            return Err(RedisError::Config(
                "connect_timeout must be greater than zero".to_string(),
            ));
        }
        if self.operation_timeout.is_zero() {
            return Err(RedisError::Config(
                "operation_timeout must be greater than zero".to_string(),
            ));
        }
        if self.pool.max_size == 0 {
            return Err(RedisError::Config(
                "pool.max_size must be greater than zero".to_string(),
            ));
        }
        if self.pool.min_idle > self.pool.max_size {
            return Err(RedisError::Config(
                "pool.min_idle cannot exceed pool.max_size".to_string(),
            ));
        }
        if self.pool.connection_timeout.is_zero() {
            return Err(RedisError::Config(
                "pool.connection_timeout must be greater than zero".to_string(),
            ));
        }
        if self.reconnect.enabled {
            if self.reconnect.initial_delay.is_zero() || self.reconnect.max_delay.is_zero() {
                return Err(RedisError::Config(
                    "reconnect delays must be greater than zero".to_string(),
                ));
            }
            if self.reconnect.initial_delay > self.reconnect.max_delay {
                return Err(RedisError::Config(
                    "reconnect.initial_delay cannot exceed reconnect.max_delay".to_string(),
                ));
            }
            if !self.reconnect.backoff_multiplier.is_finite()
                || self.reconnect.backoff_multiplier < 1.0
            {
                return Err(RedisError::Config(
                    "reconnect.backoff_multiplier must be finite and at least 1.0".to_string(),
                ));
            }
            if self.reconnect.max_attempts == Some(0) {
                return Err(RedisError::Config(
                    "reconnect.max_attempts cannot be zero when configured".to_string(),
                ));
            }
        }

        if self.sentinel.is_none() {
            let endpoints = self.parse_endpoints()?;
            if endpoints.is_empty() {
                return Err(RedisError::Config("No endpoints specified".to_string()));
            }
        }

        Ok(())
    }

    /// Parse connection endpoints from connection string
    ///
    /// Only `redis://host[:port]` URIs and comma-separated seed lists are
    /// supported. TLS URIs are rejected until TLS is implemented rather than
    /// being silently opened as plaintext TCP connections.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed, ambiguous, or unsupported connection
    /// strings.
    pub fn parse_endpoints(&self) -> RedisResult<Vec<(String, u16)>> {
        let conn_str = self.connection_string.trim();
        if conn_str.is_empty() {
            return Err(RedisError::Config("Connection string is empty".to_string()));
        }
        if conn_str.starts_with("rediss://") {
            return Err(RedisError::Config(
                "rediss:// requires TLS, which redis-oxide 0.3 does not implement".to_string(),
            ));
        }

        let addr_part = conn_str.strip_prefix("redis://").unwrap_or(conn_str);
        if addr_part.contains('@')
            || addr_part.contains('/')
            || addr_part.contains('?')
            || addr_part.contains('#')
        {
            return Err(RedisError::Config(
                "connection strings only support redis://host[:port] seed lists; configure authentication and database explicitly"
                    .to_string(),
            ));
        }

        addr_part
            .split(',')
            .map(str::trim)
            .map(Self::parse_endpoint)
            .collect()
    }

    fn parse_endpoint(endpoint: &str) -> RedisResult<(String, u16)> {
        if endpoint.is_empty() {
            return Err(RedisError::Config(
                "Connection endpoint is empty".to_string(),
            ));
        }

        if let Some(rest) = endpoint.strip_prefix('[') {
            let (host, remainder) = rest.split_once(']').ok_or_else(|| {
                RedisError::Config(format!("Invalid bracketed endpoint: {endpoint}"))
            })?;
            if host.is_empty() {
                return Err(RedisError::Config(format!("Invalid endpoint: {endpoint}")));
            }
            let port = match remainder {
                "" => 6379,
                value if value.starts_with(':') => value[1..].parse::<u16>().map_err(|_| {
                    RedisError::Config(format!("Invalid port in endpoint: {endpoint}"))
                })?,
                _ => return Err(RedisError::Config(format!("Invalid endpoint: {endpoint}"))),
            };
            return Ok((host.to_string(), port));
        }

        let colon_count = endpoint.matches(':').count();
        if colon_count > 1 {
            return Err(RedisError::Config(format!(
                "IPv6 endpoints must use brackets: {endpoint}"
            )));
        }

        if let Some((host, port)) = endpoint.rsplit_once(':') {
            if host.is_empty() {
                return Err(RedisError::Config(format!("Invalid endpoint: {endpoint}")));
            }
            let port = port
                .parse::<u16>()
                .map_err(|_| RedisError::Config(format!("Invalid port in endpoint: {endpoint}")))?;
            return Ok((host.to_string(), port));
        }

        Ok((endpoint.to_string(), 6379))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_seed_lists() {
        let config = ConnectionConfig::new("redis://localhost:6379,[::1]:6380,cache");
        assert_eq!(
            config.parse_endpoints().unwrap(),
            vec![
                ("localhost".to_string(), 6379),
                ("::1".to_string(), 6380),
                ("cache".to_string(), 6379),
            ]
        );
    }

    #[test]
    fn rejects_unsupported_or_ambiguous_uris() {
        for uri in [
            "rediss://localhost:6379",
            "redis://:secret@localhost:6379",
            "redis://localhost:6379/1",
            "redis://::1:6379",
        ] {
            assert!(
                ConnectionConfig::new(uri).parse_endpoints().is_err(),
                "{uri}"
            );
        }
    }

    #[test]
    fn validates_pool_and_reconnect_settings() {
        let mut config = ConnectionConfig::default();
        config.pool.max_size = 0;
        assert!(config.validate().is_err());

        config.pool.max_size = 1;
        config.reconnect.backoff_multiplier = 0.5;
        assert!(config.validate().is_err());
    }
}
