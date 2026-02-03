//! Connection pooling implementations
//!
//! This module provides connection pooling strategies for managing Redis connections:
//! - Multiplexed: Single connection shared across multiple tasks
//! - Pool: Multiple connections managed in a pool
//!
//! Features include:
//! - Health monitoring and automatic reconnection
//! - Connection validation before use
//! - Pool statistics for monitoring
//! - Graceful shutdown support

use crate::connection::RedisConnection;
use crate::core::{
    config::{ConnectionConfig, PoolStrategy},
    error::{RedisError, RedisResult},
    value::RespValue,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Instant;
use tokio::sync::{mpsc, Mutex, RwLock, Semaphore};
use tracing::{debug, error, info, warn};

/// Request to execute a command through the multiplexed connection
#[derive(Debug)]
struct CommandRequest {
    command: String,
    args: Vec<RespValue>,
    response_tx: tokio::sync::oneshot::Sender<RedisResult<RespValue>>,
}

/// Statistics for monitoring pool performance
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    /// Number of active connections in the pool
    pub active_connections: usize,
    /// Number of pending requests waiting to be processed
    pub pending_requests: usize,
    /// Total number of requests processed since pool creation
    pub total_requests: u64,
    /// Number of failed requests
    pub failed_requests: u64,
    /// Average response time in milliseconds
    pub average_response_time_ms: f64,
    /// Number of active worker threads
    pub worker_count: usize,
}

/// Multiplexed connection pool with health monitoring
#[derive(Debug)]
pub struct MultiplexedPool {
    command_tx: mpsc::UnboundedSender<CommandRequest>,
    stats: Arc<RwLock<PoolStats>>,
    shutdown: Arc<AtomicBool>,
}

impl MultiplexedPool {
    /// Create a new multiplexed pool with health monitoring
    pub async fn new(config: ConnectionConfig, host: String, port: u16) -> RedisResult<Self> {
        let (command_tx, mut command_rx) = mpsc::unbounded_channel::<CommandRequest>();

        let stats = Arc::new(RwLock::new(PoolStats {
            active_connections: 0,
            pending_requests: 0,
            total_requests: 0,
            failed_requests: 0,
            average_response_time_ms: 0.0,
            worker_count: 0,
        }));
        let shutdown = Arc::new(AtomicBool::new(false));

        let pool = Self {
            command_tx,
            stats,
            shutdown,
        };

        let stats_clone = pool.stats.clone();
        let shutdown_clone = pool.shutdown.clone();

        tokio::spawn(async move {
            let mut conn = match RedisConnection::connect(&host, port, config.clone()).await {
                Ok(conn) => conn,
                Err(e) => {
                    warn!("Failed to create multiplexed connection: {:?}", e);
                    return;
                }
            };

            {
                let mut stats_guard = stats_clone.write().await;
                stats_guard.active_connections = 1;
            }

            let mut last_health_check = Instant::now();
            let health_check_interval = std::time::Duration::from_secs(30);

            while !shutdown_clone.load(Ordering::SeqCst) {
                if last_health_check.elapsed() > health_check_interval {
                    if let Err(e) = conn.execute_command("PING", &[]).await {
                        warn!("Health check failed, reconnecting: {:?}", e);
                        match RedisConnection::connect(&host, port, config.clone()).await {
                            Ok(new_conn) => {
                                conn = new_conn;
                                info!("Reconnected successfully");
                            }
                            Err(e) => {
                                error!("Failed to reconnect: {:?}", e);
                                break;
                            }
                        }
                    }
                    last_health_check = Instant::now();
                }

                let request =
                    tokio::time::timeout(std::time::Duration::from_millis(100), command_rx.recv())
                        .await;

                match request {
                    Ok(Some(req)) => {
                        let start_time = Instant::now();
                        let result = conn.execute_command(&req.command, &req.args).await;
                        let response_time = start_time.elapsed();

                        let mut stats_guard = stats_clone.write().await;
                        stats_guard.total_requests += 1;
                        if result.is_err() {
                            stats_guard.failed_requests += 1;
                        }
                        let current_avg = stats_guard.average_response_time_ms;
                        let new_time = response_time.as_millis() as f64;
                        stats_guard.average_response_time_ms =
                            (current_avg * 0.9) + (new_time * 0.1);

                        let _ = req.response_tx.send(result);
                    }
                    Ok(None) => break,
                    Err(_) => {}
                }
            }

            let mut stats_guard = stats_clone.write().await;
            stats_guard.active_connections = 0;
            debug!("Multiplexed connection handler stopped");
        });

        Ok(pool)
    }

    /// Execute a command through the multiplexed connection
    pub async fn execute_command(
        &self,
        command: String,
        args: Vec<RespValue>,
    ) -> RedisResult<RespValue> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        {
            let mut stats_guard = self.stats.write().await;
            stats_guard.pending_requests += 1;
        }

        self.command_tx
            .send(CommandRequest {
                command,
                args,
                response_tx,
            })
            .map_err(|_| RedisError::Connection("Multiplexed connection closed".to_string()))?;

        let result = response_rx
            .await
            .map_err(|_| RedisError::Connection("Response channel closed".to_string()))?;

        {
            let mut stats_guard = self.stats.write().await;
            stats_guard.pending_requests = stats_guard.pending_requests.saturating_sub(1);
        }

        result
    }

    /// Get current pool statistics
    pub async fn stats(&self) -> PoolStats {
        self.stats.read().await.clone()
    }

    /// Shutdown the pool gracefully
    pub async fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        info!("Multiplexed pool shutdown initiated");
    }
}

/// Traditional connection pool with validation
#[derive(Debug)]
pub struct ConnectionPool {
    connections: Arc<RwLock<Vec<Arc<Mutex<RedisConnection>>>>>,
    semaphore: Arc<Semaphore>,
    config: ConnectionConfig,
    host: String,
    port: u16,
    stats: Arc<RwLock<PoolStats>>,
}

impl ConnectionPool {
    /// Create a new connection pool with validation
    pub async fn new(
        config: ConnectionConfig,
        host: String,
        port: u16,
        max_size: usize,
    ) -> RedisResult<Self> {
        let mut connections = Vec::new();
        let initial_size = config.pool.min_idle.min(max_size).max(1);

        for _ in 0..initial_size {
            let conn = RedisConnection::connect(&host, port, config.clone()).await?;
            connections.push(Arc::new(Mutex::new(conn)));
        }

        let stats = Arc::new(RwLock::new(PoolStats {
            active_connections: initial_size,
            pending_requests: 0,
            total_requests: 0,
            failed_requests: 0,
            average_response_time_ms: 0.0,
            worker_count: 0,
        }));

        Ok(Self {
            connections: Arc::new(RwLock::new(connections)),
            semaphore: Arc::new(Semaphore::new(max_size)),
            config,
            host,
            port,
            stats,
        })
    }

    /// Get a validated connection from the pool
    async fn get_validated_connection(&self) -> RedisResult<Arc<Mutex<RedisConnection>>> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| RedisError::Pool("Failed to acquire permit".to_string()))?;

        let conn = {
            let mut connections = self.connections.write().await;
            connections.pop()
        };

        let conn = match conn {
            Some(conn) => conn,
            None => {
                let new_conn =
                    RedisConnection::connect(&self.host, self.port, self.config.clone()).await?;
                Arc::new(Mutex::new(new_conn))
            }
        };

        {
            let mut conn_guard = conn.lock().await;
            if let Err(_) = conn_guard.execute_command("PING", &[]).await {
                match RedisConnection::connect(&self.host, self.port, self.config.clone()).await {
                    Ok(new_conn) => *conn_guard = new_conn,
                    Err(e) => return Err(e),
                }
            }
        }

        Ok(conn)
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
        let start_time = Instant::now();
        let conn = self.get_validated_connection().await?;

        let result = {
            let mut conn_guard = conn.lock().await;
            conn_guard.execute_command(&command, &args).await
        };

        self.return_connection(conn).await;

        let response_time = start_time.elapsed();
        let mut stats_guard = self.stats.write().await;
        stats_guard.total_requests += 1;
        if result.is_err() {
            stats_guard.failed_requests += 1;
        }
        let current_avg = stats_guard.average_response_time_ms;
        let new_time = response_time.as_millis() as f64;
        stats_guard.average_response_time_ms = (current_avg * 0.9) + (new_time * 0.1);

        result
    }

    /// Get current pool statistics
    pub async fn stats(&self) -> PoolStats {
        let mut stats = self.stats.read().await.clone();
        stats.active_connections = self.connections.read().await.len();
        stats
    }
}

/// Unified pool abstraction
#[derive(Debug)]
#[allow(missing_docs)]
pub enum Pool {
    Multiplexed(MultiplexedPool),
    Pool(Box<ConnectionPool>),
}

impl Pool {
    /// Create a new pool based on the configuration
    pub async fn new(config: ConnectionConfig, host: String, port: u16) -> RedisResult<Self> {
        match config.pool.strategy {
            PoolStrategy::Multiplexed => {
                let pool = MultiplexedPool::new(config, host, port).await?;
                Ok(Self::Multiplexed(pool))
            }
            PoolStrategy::Pool => {
                let pool =
                    ConnectionPool::new(config.clone(), host, port, config.pool.max_size).await?;
                Ok(Self::Pool(Box::new(pool)))
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
            Self::Multiplexed(pool) => pool.execute_command(command, args).await,
            Self::Pool(pool) => pool.execute_command(command, args).await,
        }
    }

    /// Get current pool statistics
    pub async fn stats(&self) -> PoolStats {
        match self {
            Self::Multiplexed(pool) => pool.stats().await,
            Self::Pool(pool) => pool.stats().await,
        }
    }

    /// Shutdown the pool gracefully
    pub async fn shutdown(&self) {
        match self {
            Self::Multiplexed(pool) => pool.shutdown().await,
            Self::Pool(_) => {
                info!("Connection pool shutdown");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::PoolConfig;

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

    #[tokio::test]
    async fn test_pool_stats_structure() {
        let stats = PoolStats::default();
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.total_requests, 0);
        assert!((stats.average_response_time_ms - 0.0).abs() < f64::EPSILON);
    }
}
