//! Connection pooling implementations.
//!
//! Pools never return a connection that timed out, lost its transport, or
//! failed protocol decoding. Stateful Redis modes such as transactions and
//! subscriptions use [`Pool::dedicated_connection`] instead of a shared pool.

use crate::connection::RedisConnection;
use crate::core::{
    config::{ConnectionConfig, PoolStrategy},
    error::{RedisError, RedisResult},
    value::RespValue,
};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex, OwnedSemaphorePermit, Semaphore};
use tokio::time::{sleep, timeout};
use tracing::{debug, warn};

const MULTIPLEXED_QUEUE_CAPACITY: usize = 1024;

type CommandBatch = Vec<(String, Vec<RespValue>)>;

/// Request to execute one contiguous batch through a multiplexed connection.
struct CommandRequest {
    commands: CommandBatch,
    response_tx: oneshot::Sender<RedisResult<Vec<RespValue>>>,
}

/// Multiplexed connection pool with one serialized Redis connection.
pub struct MultiplexedPool {
    command_tx: mpsc::Sender<CommandRequest>,
    config: ConnectionConfig,
    host: String,
    port: u16,
}

impl MultiplexedPool {
    /// Create a multiplexed pool after its first connection is ready.
    pub async fn new(config: ConnectionConfig, host: String, port: u16) -> RedisResult<Self> {
        config.validate()?;
        let connection = RedisConnection::connect(&host, port, config.clone()).await?;
        let (command_tx, command_rx) = mpsc::channel(MULTIPLEXED_QUEUE_CAPACITY);

        tokio::spawn(Self::run_worker(
            Some(connection),
            command_rx,
            config.clone(),
            host.clone(),
            port,
        ));

        Ok(Self {
            command_tx,
            config,
            host,
            port,
        })
    }

    async fn run_worker(
        mut connection: Option<RedisConnection>,
        mut command_rx: mpsc::Receiver<CommandRequest>,
        config: ConnectionConfig,
        host: String,
        port: u16,
    ) {
        while let Some(request) = command_rx.recv().await {
            if connection.is_none() {
                match Self::connect_with_retries(&config, &host, port).await {
                    Ok(new_connection) => connection = Some(new_connection),
                    Err(error) => {
                        let _ = request.response_tx.send(Err(error));
                        continue;
                    }
                }
            }

            let result = match connection.as_mut() {
                Some(conn) => conn.execute_pipeline(&request.commands).await,
                None => Err(RedisError::Connection(
                    "Multiplexed connection is unavailable".to_string(),
                )),
            };
            if result
                .as_ref()
                .err()
                .is_some_and(Self::invalidates_connection)
            {
                connection = None;
            }
            let _ = request.response_tx.send(result);
        }

        debug!("Multiplexed connection handler stopped");
    }

    pub(crate) async fn connect_with_retries(
        config: &ConnectionConfig,
        host: &str,
        port: u16,
    ) -> RedisResult<RedisConnection> {
        let mut attempts = 0usize;
        let mut delay = config.reconnect.initial_delay;

        loop {
            attempts = attempts.saturating_add(1);
            match RedisConnection::connect(host, port, config.clone()).await {
                Ok(connection) => return Ok(connection),
                Err(error) => {
                    let retry_limit_reached = config
                        .reconnect
                        .max_attempts
                        .is_some_and(|max_attempts| attempts >= max_attempts);
                    if !config.reconnect.enabled || retry_limit_reached {
                        return Err(error);
                    }

                    warn!(
                        "Redis connection attempt {} to {}:{} failed: {}; retrying in {:?}",
                        attempts, host, port, error, delay
                    );
                    sleep(delay).await;
                    delay = delay
                        .mul_f64(config.reconnect.backoff_multiplier)
                        .min(config.reconnect.max_delay);
                }
            }
        }
    }

    fn invalidates_connection(error: &RedisError) -> bool {
        matches!(
            error,
            RedisError::Io(_)
                | RedisError::Connection(_)
                | RedisError::Timeout
                | RedisError::Protocol(_)
        )
    }

    /// Execute a single command through the multiplexed connection.
    pub async fn execute_command(
        &self,
        command: String,
        args: Vec<RespValue>,
    ) -> RedisResult<RespValue> {
        let responses = self.execute_batch(vec![(command, args)]).await?;
        RedisConnection::into_command_result(
            responses
                .into_iter()
                .next()
                .ok_or_else(|| RedisError::Protocol("Missing command response".to_string()))?,
        )
    }

    /// Execute a contiguous batch and retain server errors per response.
    pub async fn execute_batch(&self, commands: CommandBatch) -> RedisResult<Vec<RespValue>> {
        let (response_tx, response_rx) = oneshot::channel();
        let request = CommandRequest {
            commands,
            response_tx,
        };

        timeout(self.config.operation_timeout, self.command_tx.send(request))
            .await
            .map_err(|_| RedisError::Timeout)?
            .map_err(|_| RedisError::Connection("Multiplexed connection closed".to_string()))?;

        timeout(self.config.operation_timeout, response_rx)
            .await
            .map_err(|_| RedisError::Timeout)?
            .map_err(|_| RedisError::Connection("Response channel closed".to_string()))?
    }

    /// Open an isolated connection for stateful Redis commands.
    pub async fn dedicated_connection(&self) -> RedisResult<RedisConnection> {
        RedisConnection::connect(&self.host, self.port, self.config.clone()).await
    }
}

/// Traditional bounded pool of independent Redis connections.
pub struct ConnectionPool {
    connections: Mutex<Vec<RedisConnection>>,
    semaphore: Arc<Semaphore>,
    config: ConnectionConfig,
    host: String,
    port: u16,
}

impl ConnectionPool {
    /// Create a connection pool with its configured minimum number of idle connections.
    pub async fn new(
        config: ConnectionConfig,
        host: String,
        port: u16,
        max_size: usize,
    ) -> RedisResult<Self> {
        config.validate()?;
        if max_size != config.pool.max_size {
            return Err(RedisError::Config(
                "Pool max_size must match ConnectionConfig.pool.max_size".to_string(),
            ));
        }

        let mut connections = Vec::with_capacity(config.pool.min_idle);
        for _ in 0..config.pool.min_idle {
            connections.push(RedisConnection::connect(&host, port, config.clone()).await?);
        }

        Ok(Self {
            connections: Mutex::new(connections),
            semaphore: Arc::new(Semaphore::new(max_size)),
            config,
            host,
            port,
        })
    }

    async fn get_connection(&self) -> RedisResult<(RedisConnection, OwnedSemaphorePermit)> {
        let permit = timeout(
            self.config.pool.connection_timeout,
            self.semaphore.clone().acquire_owned(),
        )
        .await
        .map_err(|_| RedisError::Timeout)?
        .map_err(|_| RedisError::Pool("Connection pool closed".to_string()))?;

        let existing = self.connections.lock().await.pop();
        let connection = match existing {
            Some(connection) => connection,
            None => {
                MultiplexedPool::connect_with_retries(&self.config, &self.host, self.port).await?
            }
        };
        Ok((connection, permit))
    }

    async fn return_connection(&self, connection: RedisConnection) {
        self.connections.lock().await.push(connection);
    }

    /// Execute a single command using one checked-out connection.
    pub async fn execute_command(
        &self,
        command: String,
        args: Vec<RespValue>,
    ) -> RedisResult<RespValue> {
        let responses = self.execute_batch(vec![(command, args)]).await?;
        RedisConnection::into_command_result(
            responses
                .into_iter()
                .next()
                .ok_or_else(|| RedisError::Protocol("Missing command response".to_string()))?,
        )
    }

    /// Execute a contiguous batch using one checked-out connection.
    pub async fn execute_batch(&self, commands: CommandBatch) -> RedisResult<Vec<RespValue>> {
        let (mut connection, permit) = self.get_connection().await?;
        let result = connection.execute_pipeline(&commands).await;
        let reusable = result
            .as_ref()
            .err()
            .is_none_or(|error| !MultiplexedPool::invalidates_connection(error));
        if reusable {
            self.return_connection(connection).await;
        }
        drop(permit);
        result
    }

    /// Open an isolated connection for stateful Redis commands.
    pub async fn dedicated_connection(&self) -> RedisResult<RedisConnection> {
        RedisConnection::connect(&self.host, self.port, self.config.clone()).await
    }
}

/// Unified pool abstraction for a multiplexed or bounded connection pool.
pub enum Pool {
    /// Multiplexed connection.
    Multiplexed(MultiplexedPool),
    /// Traditional connection pool.
    Pool(Box<ConnectionPool>),
}

impl Pool {
    /// Create a pool based on connection configuration.
    pub async fn new(config: ConnectionConfig, host: String, port: u16) -> RedisResult<Self> {
        config.validate()?;
        match config.pool.strategy {
            PoolStrategy::Multiplexed => Ok(Self::Multiplexed(
                MultiplexedPool::new(config, host, port).await?,
            )),
            PoolStrategy::Pool => Ok(Self::Pool(Box::new(
                ConnectionPool::new(config.clone(), host, port, config.pool.max_size).await?,
            ))),
        }
    }

    /// Execute a single command through the pool.
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

    /// Execute a contiguous batch and retain server errors per response.
    pub async fn execute_batch(&self, commands: CommandBatch) -> RedisResult<Vec<RespValue>> {
        match self {
            Self::Multiplexed(pool) => pool.execute_batch(commands).await,
            Self::Pool(pool) => pool.execute_batch(commands).await,
        }
    }

    /// Open an isolated connection for a transaction or subscription.
    pub async fn dedicated_connection(&self) -> RedisResult<RedisConnection> {
        match self {
            Self::Multiplexed(pool) => pool.dedicated_connection().await,
            Self::Pool(pool) => pool.dedicated_connection().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::PoolConfig;
    use std::time::Duration;

    #[test]
    fn rejects_an_invalid_pool_size() {
        let mut config = ConnectionConfig::new("redis://localhost:6379");
        config.pool.max_size = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn accepts_a_valid_pool_configuration() {
        let config = ConnectionConfig::new("redis://localhost:6379").with_pool_config(PoolConfig {
            strategy: PoolStrategy::Pool,
            max_size: 20,
            min_idle: 5,
            connection_timeout: Duration::from_secs(5),
        });
        assert!(config.validate().is_ok());
    }
}
