//! Pipeline support for batching Redis commands
//!
//! This module provides functionality to batch multiple Redis commands together
//! and execute them in a single network round-trip, improving performance
//! when executing multiple operations.
//!
//! # Examples
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ConnectionConfig::new("redis://localhost:6379");
//! let client = Client::connect(config).await?;
//!
//! // Create a pipeline
//! let mut pipeline = client.pipeline();
//!
//! // Add commands to the pipeline
//! pipeline.set("key1", "value1");
//! pipeline.set("key2", "value2");
//! pipeline.get("key1");
//! pipeline.incr("counter");
//!
//! // Execute all commands at once
//! let results = pipeline.execute().await?;
//! println!("Pipeline results: {:?}", results);
//! # Ok(())
//! # }
//! ```

use crate::core::{
    error::{RedisError, RedisResult},
    value::RespValue,
};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Trait for commands that can be used in pipelines
pub trait PipelineCommand: Send + Sync {
    /// Get the command name
    fn name(&self) -> &str;

    /// Get the command arguments
    fn args(&self) -> Vec<RespValue>;

    /// Get the key(s) involved in this command (for cluster routing)
    fn key(&self) -> Option<String>;
}

/// A pipeline for batching Redis commands
///
/// Pipeline allows you to send multiple commands to Redis in a single
/// network round-trip, which can significantly improve performance when
/// executing many operations.
pub struct Pipeline {
    commands: VecDeque<Box<dyn PipelineCommand>>,
    connection: Arc<Mutex<dyn PipelineExecutor + Send + Sync>>,
}

/// Trait for executing pipelined commands
#[async_trait::async_trait]
pub trait PipelineExecutor {
    /// Execute a batch of commands and return their results
    async fn execute_pipeline(
        &mut self,
        commands: Vec<Box<dyn PipelineCommand>>,
    ) -> RedisResult<Vec<RespValue>>;
}

impl Pipeline {
    /// Create a new pipeline
    pub fn new(connection: Arc<Mutex<dyn PipelineExecutor + Send + Sync>>) -> Self {
        Self {
            commands: VecDeque::new(),
            connection,
        }
    }

    /// Add a command to the pipeline
    pub fn add_command(&mut self, command: Box<dyn PipelineCommand>) -> &mut Self {
        self.commands.push_back(command);
        self
    }

    /// Add a SET command to the pipeline
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        use crate::commands::SetCommand;
        let cmd = SetCommand::new(key.into(), value.into());
        self.add_command(Box::new(cmd))
    }

    /// Add a GET command to the pipeline
    pub fn get(&mut self, key: impl Into<String>) -> &mut Self {
        use crate::commands::GetCommand;
        let cmd = GetCommand::new(key.into());
        self.add_command(Box::new(cmd))
    }

    /// Add a DEL command to the pipeline
    pub fn del(&mut self, keys: Vec<String>) -> &mut Self {
        use crate::commands::DelCommand;
        let cmd = DelCommand::new(keys);
        self.add_command(Box::new(cmd))
    }

    /// Add an INCR command to the pipeline
    pub fn incr(&mut self, key: impl Into<String>) -> &mut Self {
        use crate::commands::IncrCommand;
        let cmd = IncrCommand::new(key.into());
        self.add_command(Box::new(cmd))
    }

    /// Add a DECR command to the pipeline
    pub fn decr(&mut self, key: impl Into<String>) -> &mut Self {
        use crate::commands::DecrCommand;
        let cmd = DecrCommand::new(key.into());
        self.add_command(Box::new(cmd))
    }

    /// Add an INCRBY command to the pipeline
    pub fn incr_by(&mut self, key: impl Into<String>, increment: i64) -> &mut Self {
        use crate::commands::IncrByCommand;
        let cmd = IncrByCommand::new(key.into(), increment);
        self.add_command(Box::new(cmd))
    }

    /// Add a DECRBY command to the pipeline
    pub fn decr_by(&mut self, key: impl Into<String>, decrement: i64) -> &mut Self {
        use crate::commands::DecrByCommand;
        let cmd = DecrByCommand::new(key.into(), decrement);
        self.add_command(Box::new(cmd))
    }

    /// Add an EXISTS command to the pipeline
    pub fn exists(&mut self, keys: Vec<String>) -> &mut Self {
        use crate::commands::ExistsCommand;
        let cmd = ExistsCommand::new(keys);
        self.add_command(Box::new(cmd))
    }

    /// Add an EXPIRE command to the pipeline
    pub fn expire(&mut self, key: impl Into<String>, seconds: std::time::Duration) -> &mut Self {
        use crate::commands::ExpireCommand;
        let cmd = ExpireCommand::new(key.into(), seconds);
        self.add_command(Box::new(cmd))
    }

    /// Add a TTL command to the pipeline
    pub fn ttl(&mut self, key: impl Into<String>) -> &mut Self {
        use crate::commands::TtlCommand;
        let cmd = TtlCommand::new(key.into());
        self.add_command(Box::new(cmd))
    }

    // Hash commands

    /// Add an HGET command to the pipeline
    pub fn hget(&mut self, key: impl Into<String>, field: impl Into<String>) -> &mut Self {
        use crate::commands::HGetCommand;
        let cmd = HGetCommand::new(key.into(), field.into());
        self.add_command(Box::new(cmd))
    }

    /// Add an HSET command to the pipeline
    pub fn hset(
        &mut self,
        key: impl Into<String>,
        field: impl Into<String>,
        value: impl Into<String>,
    ) -> &mut Self {
        use crate::commands::HSetCommand;
        let cmd = HSetCommand::new(key.into(), field.into(), value.into());
        self.add_command(Box::new(cmd))
    }

    /// Add an HDEL command to the pipeline
    pub fn hdel(&mut self, key: impl Into<String>, fields: Vec<String>) -> &mut Self {
        use crate::commands::HDelCommand;
        let cmd = HDelCommand::new(key.into(), fields);
        self.add_command(Box::new(cmd))
    }

    /// Add an HGETALL command to the pipeline
    pub fn hgetall(&mut self, key: impl Into<String>) -> &mut Self {
        use crate::commands::HGetAllCommand;
        let cmd = HGetAllCommand::new(key.into());
        self.add_command(Box::new(cmd))
    }

    /// Add an HMGET command to the pipeline
    pub fn hmget(&mut self, key: impl Into<String>, fields: Vec<String>) -> &mut Self {
        use crate::commands::HMGetCommand;
        let cmd = HMGetCommand::new(key.into(), fields);
        self.add_command(Box::new(cmd))
    }

    /// Add an HMSET command to the pipeline
    pub fn hmset(
        &mut self,
        key: impl Into<String>,
        fields: std::collections::HashMap<String, String>,
    ) -> &mut Self {
        use crate::commands::HMSetCommand;
        let cmd = HMSetCommand::new(key.into(), fields);
        self.add_command(Box::new(cmd))
    }

    /// Add an HLEN command to the pipeline
    pub fn hlen(&mut self, key: impl Into<String>) -> &mut Self {
        use crate::commands::HLenCommand;
        let cmd = HLenCommand::new(key.into());
        self.add_command(Box::new(cmd))
    }

    /// Add an HEXISTS command to the pipeline
    pub fn hexists(&mut self, key: impl Into<String>, field: impl Into<String>) -> &mut Self {
        use crate::commands::HExistsCommand;
        let cmd = HExistsCommand::new(key.into(), field.into());
        self.add_command(Box::new(cmd))
    }

    /// Get the number of commands in the pipeline
    #[must_use]
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Check if the pipeline is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Clear all commands from the pipeline
    pub fn clear(&mut self) {
        self.commands.clear();
    }

    /// Execute all commands in the pipeline
    ///
    /// This sends all batched commands to Redis in a single network round-trip
    /// and returns their results in the same order they were added.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The pipeline is empty
    /// - Network communication fails
    /// - Any command in the pipeline fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use redis_oxide::{Client, ConnectionConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ConnectionConfig::new("redis://localhost:6379");
    /// # let client = Client::connect(config).await?;
    /// let mut pipeline = client.pipeline();
    /// pipeline.set("key1", "value1");
    /// pipeline.get("key1");
    ///
    /// let results = pipeline.execute().await?;
    /// assert_eq!(results.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute(&mut self) -> RedisResult<Vec<RespValue>> {
        if self.commands.is_empty() {
            return Err(RedisError::Protocol("Pipeline is empty".to_string()));
        }

        // Convert VecDeque to Vec for execution
        let commands: Vec<Box<dyn PipelineCommand>> = self.commands.drain(..).collect();

        // Execute the pipeline
        let mut connection = self.connection.lock().await;
        let results = connection.execute_pipeline(commands).await?;

        Ok(results)
    }

    /// Execute the pipeline and return typed results
    ///
    /// This is a convenience method that executes the pipeline and attempts
    /// to convert the results to the expected types.
    ///
    /// # Errors
    ///
    /// Returns an error if execution fails or type conversion fails.
    pub async fn execute_typed<T>(&mut self) -> RedisResult<Vec<T>>
    where
        T: TryFrom<RespValue>,
        T::Error: Into<RedisError>,
    {
        let results = self.execute().await?;
        let mut typed_results = Vec::with_capacity(results.len());

        for result in results {
            let typed_result = T::try_from(result).map_err(Into::into)?;
            typed_results.push(typed_result);
        }

        Ok(typed_results)
    }
}

/// Pipeline result wrapper for easier handling
#[derive(Debug, Clone)]
pub struct PipelineResult {
    results: Vec<RespValue>,
    index: usize,
}

impl PipelineResult {
    /// Create a new pipeline result
    #[must_use]
    pub fn new(results: Vec<RespValue>) -> Self {
        Self { results, index: 0 }
    }

    /// Get the next result from the pipeline
    ///
    /// # Errors
    ///
    /// Returns an error if there are no more results or type conversion fails.
    pub fn next<T>(&mut self) -> RedisResult<T>
    where
        T: TryFrom<RespValue>,
        T::Error: Into<RedisError>,
    {
        if self.index >= self.results.len() {
            return Err(RedisError::Protocol(
                "No more results in pipeline".to_string(),
            ));
        }

        let result = self.results[self.index].clone();
        self.index += 1;

        T::try_from(result).map_err(Into::into)
    }

    /// Get a result at a specific index
    ///
    /// # Errors
    ///
    /// Returns an error if the index is out of bounds or type conversion fails.
    pub fn get<T>(&self, index: usize) -> RedisResult<T>
    where
        T: TryFrom<RespValue>,
        T::Error: Into<RedisError>,
    {
        if index >= self.results.len() {
            return Err(RedisError::Protocol(format!(
                "Index {} out of bounds",
                index
            )));
        }

        let result = self.results[index].clone();
        T::try_from(result).map_err(Into::into)
    }

    /// Get the number of results
    #[must_use]
    pub fn len(&self) -> usize {
        self.results.len()
    }

    /// Check if there are no results
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    /// Get all results as a vector
    #[must_use]
    pub fn into_results(self) -> Vec<RespValue> {
        self.results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    struct MockExecutor {
        expected_commands: usize,
    }

    #[async_trait::async_trait]
    impl PipelineExecutor for MockExecutor {
        async fn execute_pipeline(
            &mut self,
            commands: Vec<Box<dyn PipelineCommand>>,
        ) -> RedisResult<Vec<RespValue>> {
            assert_eq!(commands.len(), self.expected_commands);

            // Return mock results
            let mut results = Vec::new();
            for _ in 0..commands.len() {
                results.push(RespValue::SimpleString("OK".to_string()));
            }
            Ok(results)
        }
    }

    #[tokio::test]
    async fn test_pipeline_creation() {
        let executor = MockExecutor {
            expected_commands: 0,
        };
        let pipeline = Pipeline::new(Arc::new(Mutex::new(executor)));

        assert!(pipeline.is_empty());
        assert_eq!(pipeline.len(), 0);
    }

    #[tokio::test]
    async fn test_pipeline_add_commands() {
        let executor = MockExecutor {
            expected_commands: 2,
        };
        let mut pipeline = Pipeline::new(Arc::new(Mutex::new(executor)));

        pipeline.set("key1", "value1");
        pipeline.get("key1");

        assert_eq!(pipeline.len(), 2);
        assert!(!pipeline.is_empty());
    }

    #[tokio::test]
    async fn test_pipeline_execute() {
        let executor = MockExecutor {
            expected_commands: 2,
        };
        let mut pipeline = Pipeline::new(Arc::new(Mutex::new(executor)));

        pipeline.set("key1", "value1");
        pipeline.get("key1");

        let results = pipeline.execute().await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(pipeline.is_empty()); // Commands should be consumed
    }

    #[tokio::test]
    async fn test_pipeline_clear() {
        let executor = MockExecutor {
            expected_commands: 0,
        };
        let mut pipeline = Pipeline::new(Arc::new(Mutex::new(executor)));

        pipeline.set("key1", "value1");
        pipeline.get("key1");
        assert_eq!(pipeline.len(), 2);

        pipeline.clear();
        assert!(pipeline.is_empty());
        assert_eq!(pipeline.len(), 0);
    }

    #[tokio::test]
    async fn test_pipeline_result() {
        let results = vec![
            RespValue::SimpleString("OK".to_string()),
            RespValue::BulkString("value1".as_bytes().into()),
            RespValue::Integer(42),
        ];

        let mut pipeline_result = PipelineResult::new(results);

        assert_eq!(pipeline_result.len(), 3);
        assert!(!pipeline_result.is_empty());

        let first: String = pipeline_result.next().unwrap();
        assert_eq!(first, "OK");

        let second: String = pipeline_result.get(1).unwrap();
        assert_eq!(second, "value1");
    }
}
