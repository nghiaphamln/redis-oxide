//! Connection management and topology detection
//!
//! This module handles low-level TCP connections to Redis servers,
//! automatic topology detection, and connection lifecycle management.

use crate::core::{
    config::{ConnectionConfig, ProtocolVersion, TopologyMode},
    error::{RedisError, RedisResult},
    value::RespValue,
};
use crate::protocol::{Resp3Decoder, RespDecoder, RespEncoder};
use bytes::{Buf, BytesMut};
use std::io::Cursor;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{debug, info};

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
    protocol: ConnectionProtocol,
}

enum ConnectionProtocol {
    Resp2,
    Resp3(Resp3Decoder),
}

impl RedisConnection {
    fn is_cluster_info(info: &str) -> bool {
        info.contains("cluster_enabled:1") || info.contains("cluster_state:ok")
    }

    /// Connect to a Redis server
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be completed.
    pub async fn connect(host: &str, port: u16, config: ConnectionConfig) -> RedisResult<Self> {
        config.validate()?;
        let addr = Self::socket_address(host, port);
        debug!("Connecting to Redis at {}", addr);

        let stream = timeout(config.connect_timeout, TcpStream::connect(&addr))
            .await
            .map_err(|_| RedisError::Timeout)?
            .map_err(|e| RedisError::Connection(format!("Failed to connect to {addr}: {e}")))?;

        let stream = if let Some(keepalive_duration) = config.tcp_keepalive {
            let socket = socket2::Socket::from(stream.into_std()?);
            let keepalive = socket2::TcpKeepalive::new().with_time(keepalive_duration);
            socket
                .set_tcp_keepalive(&keepalive)
                .map_err(|e| RedisError::Connection(format!("Failed to set TCP keepalive: {e}")))?;
            TcpStream::from_std(socket.into())?
        } else {
            stream
        };

        let mut conn = Self {
            stream,
            read_buffer: BytesMut::with_capacity(8192),
            config: config.clone(),
            protocol: ConnectionProtocol::Resp2,
        };

        if let Some(ref password) = config.password {
            conn.authenticate(password).await?;
        }
        if config.protocol_version == ProtocolVersion::Resp3 {
            conn.negotiate_resp3().await?;
        }
        if config.database != 0 {
            conn.select_database(config.database).await?;
        }

        Ok(conn)
    }

    fn socket_address(host: &str, port: u16) -> String {
        if host.contains(':') && !host.starts_with('[') {
            format!("[{host}]:{port}")
        } else {
            format!("{host}:{port}")
        }
    }

    async fn negotiate_resp3(&mut self) -> RedisResult<()> {
        self.send_command(&RespValue::Array(vec![
            RespValue::from("HELLO"),
            RespValue::from("3"),
        ]))
        .await?;
        self.protocol = ConnectionProtocol::Resp3(Resp3Decoder::new());
        match self.read_response_with_timeout().await? {
            RespValue::Error(message)
                if message.contains("unknown command")
                    || message.contains("Unsupported protocol version") =>
            {
                self.protocol = ConnectionProtocol::Resp2;
                debug!("Redis server does not support RESP3; using RESP2");
                Ok(())
            }
            RespValue::Error(message) => Err(RedisError::Server(message)),
            _ => Ok(()),
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

    /// Send a command to the server
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be completed.
    pub async fn send_command(&mut self, command: &RespValue) -> RedisResult<()> {
        let mut buffer = BytesMut::new();
        RespEncoder::encode(command, &mut buffer)?;
        timeout(
            self.config.operation_timeout,
            self.stream.write_all(&buffer),
        )
        .await
        .map_err(|_| RedisError::Timeout)?
        .map_err(RedisError::Io)?;
        Ok(())
    }

    /// Execute a command and return the response
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be completed.
    pub async fn execute_command(
        &mut self,
        command: &str,
        args: &[RespValue],
    ) -> RedisResult<RespValue> {
        let responses = self
            .execute_pipeline(&[(command.to_string(), args.to_vec())])
            .await?;
        Self::into_command_result(
            responses
                .into_iter()
                .next()
                .ok_or_else(|| RedisError::Protocol("Missing command response".to_string()))?,
        )
    }

    /// Execute several commands on this connection without interleaving.
    ///
    /// Commands are fully written before responses are read, preserving Redis
    /// pipeline ordering. Server errors are returned as response values so a
    /// caller can retain per-command pipeline results.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be completed.
    pub async fn execute_pipeline(
        &mut self,
        commands: &[(String, Vec<RespValue>)],
    ) -> RedisResult<Vec<RespValue>> {
        if commands.is_empty() {
            return Ok(Vec::new());
        }

        let mut encoded = BytesMut::new();
        for (command, args) in commands {
            encoded.extend_from_slice(&RespEncoder::encode_command(command, args)?);
        }

        timeout(
            self.config.operation_timeout,
            self.stream.write_all(&encoded),
        )
        .await
        .map_err(|_| RedisError::Timeout)?
        .map_err(RedisError::Io)?;

        let mut responses = Vec::with_capacity(commands.len());
        for _ in commands {
            let response = timeout(self.config.operation_timeout, self.read_response())
                .await
                .map_err(|_| RedisError::Timeout)??;
            responses.push(response);
        }
        Ok(responses)
    }

    /// Turn a raw Redis response into the normal single-command result.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be completed.
    pub fn into_command_result(response: RespValue) -> RedisResult<RespValue> {
        if let RespValue::Error(ref msg) = response {
            if let Some(redirect_error) = RedisError::parse_redirect(msg) {
                return Err(redirect_error);
            }
            return Err(RedisError::Server(msg.clone()));
        }
        Ok(response)
    }

    /// Read a complete RESP response from the connection
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be completed.
    pub async fn read_response(&mut self) -> RedisResult<RespValue> {
        if matches!(self.protocol, ConnectionProtocol::Resp3(_)) {
            return self.read_resp3_response().await;
        }

        loop {
            // Try to decode from existing buffer
            let mut cursor = Cursor::new(&self.read_buffer[..]);
            if let Some(value) = RespDecoder::decode(&mut cursor)? {
                let pos = usize::try_from(cursor.position()).map_err(|_| {
                    RedisError::Protocol("RESP cursor position exceeds platform size".to_string())
                })?;
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

    async fn read_resp3_response(&mut self) -> RedisResult<RespValue> {
        loop {
            let buffered_result = match &mut self.protocol {
                ConnectionProtocol::Resp3(codec) => codec.try_decode(&[]),
                ConnectionProtocol::Resp2 => unreachable!("RESP3 reader requires RESP3 codec"),
            }?;
            if let Some(value) = buffered_result {
                return Ok(value.into());
            }

            let mut chunk = BytesMut::with_capacity(8192);
            let read = self.stream.read_buf(&mut chunk).await?;
            if read == 0 {
                return Err(RedisError::Connection(
                    "Connection closed by server".to_string(),
                ));
            }
            let incoming_result = match &mut self.protocol {
                ConnectionProtocol::Resp3(codec) => codec.try_decode(&chunk),
                ConnectionProtocol::Resp2 => unreachable!("RESP3 reader requires RESP3 codec"),
            }?;
            if let Some(value) = incoming_result {
                return Ok(value.into());
            }
        }
    }

    /// Read a response using the configured operation timeout.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be completed.
    pub async fn read_response_with_timeout(&mut self) -> RedisResult<RespValue> {
        timeout(self.config.operation_timeout, self.read_response())
            .await
            .map_err(|_| RedisError::Timeout)?
    }

    /// Detect the topology type of the Redis server
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be completed.
    pub async fn detect_topology(&mut self) -> RedisResult<TopologyType> {
        info!("Detecting Redis topology");

        // Try CLUSTER INFO command
        match self
            .execute_command("CLUSTER", &[RespValue::from("INFO")])
            .await
        {
            Ok(RespValue::BulkString(data)) => {
                let info_str = String::from_utf8(data.to_vec())
                    .map_err(|e| RedisError::Protocol(format!("Invalid UTF-8: {e}")))?;

                // Parse cluster_state
                if Self::is_cluster_info(&info_str) {
                    info!("Detected Redis Cluster");
                    return Ok(TopologyType::Cluster);
                }
            }
            Ok(RespValue::SimpleString(info_str)) if Self::is_cluster_info(&info_str) => {
                // Parse cluster_state
                info!("Detected Redis Cluster");
                return Ok(TopologyType::Cluster);
            }
            Err(RedisError::Server(ref e))
                if e.contains("command not supported")
                    || e.contains("unknown command")
                    || e.contains("disabled") =>
            {
                info!("Detected Standalone Redis (CLUSTER command not available)");
                return Ok(TopologyType::Standalone);
            }
            Err(e) => return Err(e),
            _ => {}
        }

        info!("Detected Standalone Redis");
        Ok(TopologyType::Standalone)
    }

    /// Select a database (only works in standalone mode)
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be completed.
    pub async fn select_database(&mut self, db: u8) -> RedisResult<()> {
        let response = self
            .execute_command("SELECT", &[RespValue::from(i64::from(db))])
            .await?;

        match response {
            RespValue::SimpleString(ref s) if s == "OK" => Ok(()),
            RespValue::Error(e) => Err(RedisError::Server(e)),
            _ => Err(RedisError::UnexpectedResponse(format!("{response:?}"))),
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
    #[must_use]
    pub const fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            topology: None,
        }
    }

    /// Get or detect the topology type
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be completed.
    pub async fn get_topology(&mut self) -> RedisResult<TopologyType> {
        if let Some(topology) = self.topology {
            return Ok(topology);
        }

        self.config.validate()?;

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
                let endpoints = self.config.parse_endpoints()?;
                let mut last_error = None;
                for (host, port) in endpoints {
                    match RedisConnection::connect(&host, port, self.config.clone()).await {
                        Ok(mut conn) => match conn.detect_topology().await {
                            Ok(topology) => {
                                self.topology = Some(topology);
                                return Ok(topology);
                            }
                            Err(error) => last_error = Some(error),
                        },
                        Err(error) => last_error = Some(error),
                    }
                }
                Err(last_error
                    .unwrap_or_else(|| RedisError::Config("No endpoints specified".to_string())))
            }
        }
    }

    /// Create a new connection to the specified host and port
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be completed.
    pub async fn create_connection(&self, host: &str, port: u16) -> RedisResult<RedisConnection> {
        RedisConnection::connect(host, port, self.config.clone()).await
    }

    /// Get the configuration
    #[must_use]
    pub const fn config(&self) -> &ConnectionConfig {
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
