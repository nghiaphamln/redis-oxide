//! Connection management and topology detection
//!
//! This module handles low-level TCP connections to Redis servers,
//! automatic topology detection, and connection lifecycle management.

use crate::core::{
    config::{ConnectionConfig, TopologyMode},
    error::{RedisError, RedisResult},
    value::RespValue,
};
use crate::protocol::{RespDecoder, RespEncoder};
use bytes::{Buf, BytesMut};
use std::io::Cursor;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{debug, info, warn};

/// Type of Redis topology
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopologyType {
    /// Standalone Redis server
    Standalone,
    /// Redis Cluster
    Cluster,
}

/// A connection to a Redis server
pub struct RedisConnection {
    stream: TcpStream,
    read_buffer: BytesMut,
    config: ConnectionConfig,
}

impl RedisConnection {
    /// Connect to a Redis server
    pub async fn connect(host: &str, port: u16, config: ConnectionConfig) -> RedisResult<Self> {
        let addr = format!("{}:{}", host, port);
        debug!("Connecting to Redis at {}", addr);

        let stream = timeout(config.connect_timeout, TcpStream::connect(&addr))
            .await
            .map_err(|_| RedisError::Timeout)?
            .map_err(|e| RedisError::Connection(format!("Failed to connect to {}: {}", addr, e)))?;

        // Set TCP keepalive if configured
        if let Some(keepalive_duration) = config.tcp_keepalive {
            let socket = socket2::Socket::from(stream.into_std()?);
            let keepalive = socket2::TcpKeepalive::new().with_time(keepalive_duration);
            socket.set_tcp_keepalive(&keepalive).map_err(|e| {
                RedisError::Connection(format!("Failed to set TCP keepalive: {}", e))
            })?;
            let stream = TcpStream::from_std(socket.into())?;

            let mut conn = Self {
                stream,
                read_buffer: BytesMut::with_capacity(8192),
                config: config.clone(),
            };

            // Authenticate if password is provided
            if let Some(ref password) = config.password {
                conn.authenticate(password).await?;
            }

            Ok(conn)
        } else {
            let mut conn = Self {
                stream,
                read_buffer: BytesMut::with_capacity(8192),
                config: config.clone(),
            };

            // Authenticate if password is provided
            if let Some(ref password) = config.password {
                conn.authenticate(password).await?;
            }

            Ok(conn)
        }
    }

    /// Authenticate with the Redis server
    async fn authenticate(&mut self, password: &str) -> RedisResult<()> {
        debug!("Authenticating with Redis server");
        let response = self
            .execute_command("AUTH", &[RespValue::from(password)])
            .await?;

        match response {
            RespValue::SimpleString(ref s) if s == "OK" => Ok(()),
            RespValue::Error(e) => Err(RedisError::Auth(e)),
            _ => Err(RedisError::Auth(
                "Unexpected authentication response".to_string(),
            )),
        }
    }

    /// Execute a command and return the response
    pub async fn execute_command(
        &mut self,
        command: &str,
        args: &[RespValue],
    ) -> RedisResult<RespValue> {
        // Encode command
        let encoded = RespEncoder::encode_command(command, args)?;

        // Send command with timeout
        timeout(
            self.config.operation_timeout,
            self.stream.write_all(&encoded),
        )
        .await
        .map_err(|_| RedisError::Timeout)?
        .map_err(RedisError::Io)?;

        // Read response with timeout
        let response = timeout(self.config.operation_timeout, self.read_response())
            .await
            .map_err(|_| RedisError::Timeout)??;

        // Check if response is an error and parse redirects
        if let RespValue::Error(ref msg) = response {
            if let Some(redirect_error) = RedisError::parse_redirect(msg) {
                return Err(redirect_error);
            }
            return Err(RedisError::Server(msg.clone()));
        }

        Ok(response)
    }

    /// Read a complete RESP response from the connection
    async fn read_response(&mut self) -> RedisResult<RespValue> {
        loop {
            // Try to decode from existing buffer
            let mut cursor = Cursor::new(&self.read_buffer[..]);
            if let Some(value) = RespDecoder::decode(&mut cursor)? {
                let pos = cursor.position() as usize;
                self.read_buffer.advance(pos);
                return Ok(value);
            }

            // Need more data - read from socket
            let n = self.stream.read_buf(&mut self.read_buffer).await?;
            if n == 0 {
                return Err(RedisError::Connection(
                    "Connection closed by server".to_string(),
                ));
            }
        }
    }

    /// Detect the topology type of the Redis server
    pub async fn detect_topology(&mut self) -> RedisResult<TopologyType> {
        info!("Detecting Redis topology");

        // Try CLUSTER INFO command
        match self
            .execute_command("CLUSTER", &[RespValue::from("INFO")])
            .await
        {
            Ok(RespValue::BulkString(data)) => {
                let info_str = String::from_utf8(data.to_vec())
                    .map_err(|e| RedisError::Protocol(format!("Invalid UTF-8: {}", e)))?;

                // Parse cluster_state
                if info_str.contains("cluster_enabled:1") || info_str.contains("cluster_state:ok") {
                    info!("Detected Redis Cluster");
                    return Ok(TopologyType::Cluster);
                }
            }
            Ok(RespValue::SimpleString(info_str)) => {
                // Parse cluster_state
                if info_str.contains("cluster_enabled:1") || info_str.contains("cluster_state:ok") {
                    info!("Detected Redis Cluster");
                    return Ok(TopologyType::Cluster);
                }
            }
            Ok(RespValue::Error(ref e))
                if e.contains("command not supported")
                    || e.contains("unknown command")
                    || e.contains("disabled") =>
            {
                // Cluster commands not available - this is standalone
                info!("Detected Standalone Redis (CLUSTER command not available)");
                return Ok(TopologyType::Standalone);
            }
            Err(RedisError::Server(ref e))
                if e.contains("command not supported")
                    || e.contains("unknown command")
                    || e.contains("disabled") =>
            {
                info!("Detected Standalone Redis (CLUSTER command not available)");
                return Ok(TopologyType::Standalone);
            }
            Err(e) => {
                warn!("Error detecting topology: {:?}, assuming standalone", e);
                return Ok(TopologyType::Standalone);
            }
            _ => {}
        }

        info!("Detected Standalone Redis");
        Ok(TopologyType::Standalone)
    }

    /// Select a database (only works in standalone mode)
    pub async fn select_database(&mut self, db: u8) -> RedisResult<()> {
        let response = self
            .execute_command("SELECT", &[RespValue::from(db as i64)])
            .await?;

        match response {
            RespValue::SimpleString(ref s) if s == "OK" => Ok(()),
            RespValue::Error(e) => Err(RedisError::Server(e)),
            _ => Err(RedisError::UnexpectedResponse(format!("{:?}", response))),
        }
    }
}

/// Connection manager that handles topology detection and connection creation
pub struct ConnectionManager {
    config: ConnectionConfig,
    topology: Option<TopologyType>,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            topology: None,
        }
    }

    /// Get or detect the topology type
    pub async fn get_topology(&mut self) -> RedisResult<TopologyType> {
        if let Some(topology) = self.topology {
            return Ok(topology);
        }

        // Check if topology mode is forced
        match self.config.topology_mode {
            TopologyMode::Standalone => {
                self.topology = Some(TopologyType::Standalone);
                Ok(TopologyType::Standalone)
            }
            TopologyMode::Cluster => {
                self.topology = Some(TopologyType::Cluster);
                Ok(TopologyType::Cluster)
            }
            TopologyMode::Auto => {
                // Auto-detect
                let endpoints = self.config.parse_endpoints();
                if endpoints.is_empty() {
                    return Err(RedisError::Config("No endpoints specified".to_string()));
                }

                let (host, port) = &endpoints[0];
                let mut conn = RedisConnection::connect(host, *port, self.config.clone()).await?;
                let topology = conn.detect_topology().await?;
                self.topology = Some(topology);
                Ok(topology)
            }
        }
    }

    /// Create a new connection to the specified host and port
    pub async fn create_connection(&self, host: &str, port: u16) -> RedisResult<RedisConnection> {
        RedisConnection::connect(host, port, self.config.clone()).await
    }

    /// Get the configuration
    pub fn config(&self) -> &ConnectionConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_manager_creation() {
        let config = ConnectionConfig::new("redis://localhost:6379");
        let manager = ConnectionManager::new(config);
        assert!(manager.topology.is_none());
    }

    #[test]
    fn test_forced_topology() {
        let config = ConnectionConfig::new("redis://localhost:6379")
            .with_topology_mode(TopologyMode::Standalone);
        let manager = ConnectionManager::new(config);

        // This would normally require async, but we can test the logic
        assert_eq!(manager.config.topology_mode, TopologyMode::Standalone);
    }
}
