//! Connection pooling implementations
//!
//! This module provides two strategies for managing Redis connections:
//! - Multiplexed: Single connection shared across multiple tasks
//! - Pool: Multiple connections managed in a pool

use crate::connection::RedisConnection;
use redis_oxide_core::{
    config::{ConnectionConfig, PoolStrategy},
    error::{RedisError, RedisResult},
    value::RespValue,
};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock, Semaphore};
use tracing::{debug, warn};

/// Request to execute a command through the multiplexed connection
struct CommandRequest {
    command: String,
    args: Vec<RespValue>,
    response_tx: tokio::sync::oneshot::Sender<RedisResult<RespValue>>,
}

/// Multiplexed connection pool - uses a single connection with mpsc channel
pub struct MultiplexedPool {
    command_tx: mpsc::UnboundedSender<CommandRequest>,
}

impl MultiplexedPool {
    /// Create a new multiplexed pool
    pub async fn new(config: ConnectionConfig, host: String, port: u16) -> RedisResult<Self> {
        let (command_tx, mut command_rx) = mpsc::unbounded_channel::<CommandRequest>();

        // Spawn background task to handle commands
        tokio::spawn(async move {
            let mut conn = match RedisConnection::connect(&host, port, config.clone()).await {
                Ok(conn) => conn,
                Err(e) => {
                    warn!("Failed to create multiplexed connection: {:?}", e);
                    return;
                }
            };

            while let Some(req) = command_rx.recv().await {
                let result = conn.execute_command(&req.command, &req.args).await;
                // Ignore send errors - client may have dropped the receiver
                let _ = req.response_tx.send(result);
            }

            debug!("Multiplexed connection handler stopped");
        });

        Ok(Self { command_tx })
    }

    /// Execute a command through the multiplexed connection
    pub async fn execute_command(
        &self,
        command: String,
        args: Vec<RespValue>,
    ) -> RedisResult<RespValue> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        self.command_tx
            .send(CommandRequest {
                command,
                args,
                response_tx,
            })
            .map_err(|_| RedisError::Connection("Multiplexed connection closed".to_string()))?;

        response_rx
            .await
            .map_err(|_| RedisError::Connection("Response channel closed".to_string()))?
    }
}

/// Traditional connection pool with multiple connections
pub struct ConnectionPool {
    connections: Arc<RwLock<Vec<Arc<Mutex<RedisConnection>>>>>,
    semaphore: Arc<Semaphore>,
    config: ConnectionConfig,
    host: String,
    port: u16,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub async fn new(
        config: ConnectionConfig,
        host: String,
        port: u16,
        max_size: usize,
    ) -> RedisResult<Self> {
        let mut connections = Vec::new();

        // Create initial connections (at least 1)
        let initial_size = config.pool.min_idle.min(max_size).max(1);
        for _ in 0..initial_size {
            let conn = RedisConnection::connect(&host, port, config.clone()).await?;
            connections.push(Arc::new(Mutex::new(conn)));
        }

        Ok(Self {
            connections: Arc::new(RwLock::new(connections)),
            semaphore: Arc::new(Semaphore::new(max_size)),
            config,
            host,
            port,
        })
    }

    /// Get a connection from the pool
    async fn get_connection(&self) -> RedisResult<Arc<Mutex<RedisConnection>>> {
        // Acquire semaphore permit
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| RedisError::Pool("Failed to acquire permit".to_string()))?;

        // Try to get an existing connection
        {
            let mut connections = self.connections.write().await;
            if let Some(conn) = connections.pop() {
                return Ok(conn);
            }
        }

        // Create a new connection if none available
        let conn = RedisConnection::connect(&self.host, self.port, self.config.clone()).await?;
        Ok(Arc::new(Mutex::new(conn)))
    }

    /// Return a connection to the pool
    async fn return_connection(&self, conn: Arc<Mutex<RedisConnection>>) {
        let mut connections = self.connections.write().await;
        connections.push(conn);
    }

    /// Execute a command using a connection from the pool
    pub async fn execute_command(
        &self,
        command: String,
        args: Vec<RespValue>,
    ) -> RedisResult<RespValue> {
        let conn = self.get_connection().await?;
        let result = {
            let mut conn_guard = conn.lock().await;
            conn_guard.execute_command(&command, &args).await
        };

        // Return connection to pool
        self.return_connection(conn).await;

        result
    }
}

/// Unified pool abstraction that can be either multiplexed or traditional pool
pub enum Pool {
    /// Multiplexed connection
    Multiplexed(MultiplexedPool),
    /// Traditional connection pool
    Pool(ConnectionPool),
}

impl Pool {
    /// Create a new pool based on the configuration
    pub async fn new(config: ConnectionConfig, host: String, port: u16) -> RedisResult<Self> {
        match config.pool.strategy {
            PoolStrategy::Multiplexed => {
                let pool = MultiplexedPool::new(config, host, port).await?;
                Ok(Pool::Multiplexed(pool))
            }
            PoolStrategy::Pool => {
                let pool =
                    ConnectionPool::new(config.clone(), host, port, config.pool.max_size).await?;
                Ok(Pool::Pool(pool))
            }
        }
    }

    /// Execute a command through the pool
    pub async fn execute_command(
        &self,
        command: String,
        args: Vec<RespValue>,
    ) -> RedisResult<RespValue> {
        match self {
            Pool::Multiplexed(pool) => pool.execute_command(command, args).await,
            Pool::Pool(pool) => pool.execute_command(command, args).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use redis_oxide_core::config::PoolConfig;

    #[test]
    fn test_pool_config() {
        let config = ConnectionConfig::new("redis://localhost:6379");
        assert_eq!(config.pool.strategy, PoolStrategy::Multiplexed);
    }

    #[test]
    fn test_custom_pool_config() {
        let mut config = ConnectionConfig::new("redis://localhost:6379");
        config.pool = PoolConfig {
            strategy: PoolStrategy::Pool,
            max_size: 20,
            min_idle: 5,
            ..Default::default()
        };

        assert_eq!(config.pool.strategy, PoolStrategy::Pool);
        assert_eq!(config.pool.max_size, 20);
    }
}
