//! Transaction support for Redis
//!
//! This module provides functionality for Redis transactions using MULTI/EXEC/WATCH/DISCARD.
//! Redis transactions allow you to execute a group of commands atomically.
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
//! // Start a transaction
//! let mut transaction = client.transaction().await?;
//!
//! // Add commands to the transaction
//! transaction.set("key1", "value1");
//! transaction.set("key2", "value2");
//! transaction.incr("counter");
//!
//! // Execute the transaction
//! let results = transaction.exec().await?;
//! println!("Transaction results: {:?}", results);
//! # Ok(())
//! # }
//! ```

use crate::commands::Command;
use crate::core::{
    error::{RedisError, RedisResult},
    value::RespValue,
};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A Redis transaction that executes commands atomically
pub struct Transaction {
    commands: VecDeque<Box<dyn TransactionCommand>>,
    connection: Arc<Mutex<dyn TransactionExecutor + Send + Sync>>,
    watched_keys: Vec<String>,
    is_started: bool,
}

/// Trait for commands that can be used in transactions
pub trait TransactionCommand: Send + Sync {
    /// Get the command name
    fn name(&self) -> &str;
    
    /// Get the command arguments
    fn args(&self) -> Vec<RespValue>;
    
    /// Get the key(s) involved in this command (for WATCH)
    fn key(&self) -> Option<String>;
}

/// Trait for executing transactions
#[async_trait::async_trait]
pub trait TransactionExecutor {
    /// Start a transaction with MULTI
    async fn multi(&mut self) -> RedisResult<()>;
    
    /// Execute a command in the transaction
    async fn queue_command(&mut self, command: Box<dyn TransactionCommand>) -> RedisResult<()>;
    
    /// Execute the transaction with EXEC
    async fn exec(&mut self) -> RedisResult<Vec<RespValue>>;
    
    /// Discard the transaction with DISCARD
    async fn discard(&mut self) -> RedisResult<()>;
    
    /// Watch keys for changes
    async fn watch(&mut self, keys: Vec<String>) -> RedisResult<()>;
    
    /// Unwatch all keys
    async fn unwatch(&mut self) -> RedisResult<()>;
}

impl Transaction {
    /// Create a new transaction
    pub fn new(connection: Arc<Mutex<dyn TransactionExecutor + Send + Sync>>) -> Self {
        Self {
            commands: VecDeque::new(),
            connection,
            watched_keys: Vec::new(),
            is_started: false,
        }
    }

    /// Watch keys for changes before starting the transaction
    ///
    /// If any watched key is modified before EXEC, the transaction will be discarded.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use redis_oxide::{Client, ConnectionConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ConnectionConfig::new("redis://localhost:6379");
    /// # let client = Client::connect(config).await?;
    /// let mut transaction = client.transaction().await?;
    /// 
    /// // Watch keys before starting transaction
    /// transaction.watch(vec!["balance".to_string(), "account".to_string()]).await?;
    /// 
    /// // Add commands
    /// transaction.set("balance", "100");
    /// transaction.incr("account");
    /// 
    /// // Execute - will fail if watched keys were modified
    /// let results = transaction.exec().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn watch(&mut self, keys: Vec<String>) -> RedisResult<()> {
        if self.is_started {
            return Err(RedisError::Protocol("Cannot WATCH after MULTI".to_string()));
        }

        let mut connection = self.connection.lock().await;
        connection.watch(keys.clone()).await?;
        self.watched_keys.extend(keys);
        Ok(())
    }

    /// Unwatch all previously watched keys
    pub async fn unwatch(&mut self) -> RedisResult<()> {
        let mut connection = self.connection.lock().await;
        connection.unwatch().await?;
        self.watched_keys.clear();
        Ok(())
    }

    /// Add a command to the transaction
    pub fn add_command(&mut self, command: Box<dyn TransactionCommand>) -> &mut Self {
        self.commands.push_back(command);
        self
    }

    /// Add a SET command to the transaction
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        use crate::commands::SetCommand;
        let cmd = SetCommand::new(key.into(), value.into());
        self.add_command(Box::new(cmd))
    }

    /// Add a GET command to the transaction
    pub fn get(&mut self, key: impl Into<String>) -> &mut Self {
        use crate::commands::GetCommand;
        let cmd = GetCommand::new(key.into());
        self.add_command(Box::new(cmd))
    }

    /// Add a DEL command to the transaction
    pub fn del(&mut self, keys: Vec<String>) -> &mut Self {
        use crate::commands::DelCommand;
        let cmd = DelCommand::new(keys);
        self.add_command(Box::new(cmd))
    }

    /// Add an INCR command to the transaction
    pub fn incr(&mut self, key: impl Into<String>) -> &mut Self {
        use crate::commands::IncrCommand;
        let cmd = IncrCommand::new(key.into());
        self.add_command(Box::new(cmd))
    }

    /// Add a DECR command to the transaction
    pub fn decr(&mut self, key: impl Into<String>) -> &mut Self {
        use crate::commands::DecrCommand;
        let cmd = DecrCommand::new(key.into());
        self.add_command(Box::new(cmd))
    }

    /// Add an INCRBY command to the transaction
    pub fn incr_by(&mut self, key: impl Into<String>, increment: i64) -> &mut Self {
        use crate::commands::IncrByCommand;
        let cmd = IncrByCommand::new(key.into(), increment);
        self.add_command(Box::new(cmd))
    }

    /// Add a DECRBY command to the transaction
    pub fn decr_by(&mut self, key: impl Into<String>, decrement: i64) -> &mut Self {
        use crate::commands::DecrByCommand;
        let cmd = DecrByCommand::new(key.into(), decrement);
        self.add_command(Box::new(cmd))
    }

    /// Add an EXISTS command to the transaction
    pub fn exists(&mut self, keys: Vec<String>) -> &mut Self {
        use crate::commands::ExistsCommand;
        let cmd = ExistsCommand::new(keys);
        self.add_command(Box::new(cmd))
    }

    /// Add an EXPIRE command to the transaction
    pub fn expire(&mut self, key: impl Into<String>, seconds: std::time::Duration) -> &mut Self {
        use crate::commands::ExpireCommand;
        let cmd = ExpireCommand::new(key.into(), seconds);
        self.add_command(Box::new(cmd))
    }

    /// Add a TTL command to the transaction
    pub fn ttl(&mut self, key: impl Into<String>) -> &mut Self {
        use crate::commands::TtlCommand;
        let cmd = TtlCommand::new(key.into());
        self.add_command(Box::new(cmd))
    }

    // Hash commands

    /// Add an HGET command to the transaction
    pub fn hget(&mut self, key: impl Into<String>, field: impl Into<String>) -> &mut Self {
        use crate::commands::HGetCommand;
        let cmd = HGetCommand::new(key.into(), field.into());
        self.add_command(Box::new(cmd))
    }

    /// Add an HSET command to the transaction
    pub fn hset(&mut self, key: impl Into<String>, field: impl Into<String>, value: impl Into<String>) -> &mut Self {
        use crate::commands::HSetCommand;
        let cmd = HSetCommand::new(key.into(), field.into(), value.into());
        self.add_command(Box::new(cmd))
    }

    /// Get the number of commands in the transaction
    #[must_use]
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Check if the transaction is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Clear all commands from the transaction
    pub fn clear(&mut self) {
        self.commands.clear();
    }

    /// Execute the transaction
    ///
    /// This sends MULTI, queues all commands, and then executes them with EXEC.
    /// Returns the results of all commands in the same order they were added.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The transaction is empty
    /// - Network communication fails
    /// - Any watched key was modified (returns empty result)
    /// - Any command in the transaction fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use redis_oxide::{Client, ConnectionConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ConnectionConfig::new("redis://localhost:6379");
    /// # let client = Client::connect(config).await?;
    /// let mut transaction = client.transaction().await?;
    /// transaction.set("key1", "value1");
    /// transaction.get("key1");
    /// 
    /// let results = transaction.exec().await?;
    /// assert_eq!(results.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn exec(&mut self) -> RedisResult<Vec<RespValue>> {
        if self.commands.is_empty() {
            return Err(RedisError::Protocol("Transaction is empty".to_string()));
        }

        let mut connection = self.connection.lock().await;
        
        // Start transaction if not already started
        if !self.is_started {
            connection.multi().await?;
            self.is_started = true;
        }

        // Queue all commands
        let commands: Vec<Box<dyn TransactionCommand>> = self.commands.drain(..).collect();
        for command in commands {
            connection.queue_command(command).await?;
        }

        // Execute the transaction
        let results = connection.exec().await?;
        self.is_started = false;
        
        Ok(results)
    }

    /// Discard the transaction
    ///
    /// This cancels the transaction and discards all queued commands.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use redis_oxide::{Client, ConnectionConfig};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ConnectionConfig::new("redis://localhost:6379");
    /// # let client = Client::connect(config).await?;
    /// let mut transaction = client.transaction().await?;
    /// transaction.set("key1", "value1");
    /// transaction.set("key2", "value2");
    /// 
    /// // Cancel the transaction
    /// transaction.discard().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn discard(&mut self) -> RedisResult<()> {
        let mut connection = self.connection.lock().await;
        connection.discard().await?;
        self.commands.clear();
        self.is_started = false;
        Ok(())
    }
}

/// Transaction result wrapper for easier handling
#[derive(Debug, Clone)]
pub struct TransactionResult {
    results: Vec<RespValue>,
    index: usize,
}

impl TransactionResult {
    /// Create a new transaction result
    #[must_use]
    pub fn new(results: Vec<RespValue>) -> Self {
        Self { results, index: 0 }
    }

    /// Get the next result from the transaction
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
            return Err(RedisError::Protocol("No more results in transaction".to_string()));
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
            return Err(RedisError::Protocol(format!("Index {} out of bounds", index)));
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

// Implement TransactionCommand for all command types
impl TransactionCommand for crate::commands::GetCommand {
    fn name(&self) -> &str {
        self.command_name()
    }
    
    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }
    
    fn key(&self) -> Option<String> {
        Some(self.keys()[0].iter().map(|&b| b as char).collect())
    }
}

impl TransactionCommand for crate::commands::SetCommand {
    fn name(&self) -> &str {
        self.command_name()
    }
    
    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }
    
    fn key(&self) -> Option<String> {
        Some(self.keys()[0].iter().map(|&b| b as char).collect())
    }
}

impl TransactionCommand for crate::commands::DelCommand {
    fn name(&self) -> &str {
        self.command_name()
    }
    
    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }
    
    fn key(&self) -> Option<String> {
        if let Some(first_key) = self.keys().first() {
            Some(first_key.iter().map(|&b| b as char).collect())
        } else {
            None
        }
    }
}

impl TransactionCommand for crate::commands::IncrCommand {
    fn name(&self) -> &str {
        self.command_name()
    }
    
    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }
    
    fn key(&self) -> Option<String> {
        Some(self.keys()[0].iter().map(|&b| b as char).collect())
    }
}

impl TransactionCommand for crate::commands::DecrCommand {
    fn name(&self) -> &str {
        self.command_name()
    }
    
    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }
    
    fn key(&self) -> Option<String> {
        Some(self.keys()[0].iter().map(|&b| b as char).collect())
    }
}

impl TransactionCommand for crate::commands::IncrByCommand {
    fn name(&self) -> &str {
        self.command_name()
    }
    
    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }
    
    fn key(&self) -> Option<String> {
        Some(self.keys()[0].iter().map(|&b| b as char).collect())
    }
}

impl TransactionCommand for crate::commands::DecrByCommand {
    fn name(&self) -> &str {
        self.command_name()
    }
    
    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }
    
    fn key(&self) -> Option<String> {
        Some(self.keys()[0].iter().map(|&b| b as char).collect())
    }
}

impl TransactionCommand for crate::commands::ExistsCommand {
    fn name(&self) -> &str {
        self.command_name()
    }
    
    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }
    
    fn key(&self) -> Option<String> {
        if let Some(first_key) = self.keys().first() {
            Some(first_key.iter().map(|&b| b as char).collect())
        } else {
            None
        }
    }
}

impl TransactionCommand for crate::commands::ExpireCommand {
    fn name(&self) -> &str {
        self.command_name()
    }
    
    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }
    
    fn key(&self) -> Option<String> {
        Some(self.keys()[0].iter().map(|&b| b as char).collect())
    }
}

impl TransactionCommand for crate::commands::TtlCommand {
    fn name(&self) -> &str {
        self.command_name()
    }
    
    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }
    
    fn key(&self) -> Option<String> {
        Some(self.keys()[0].iter().map(|&b| b as char).collect())
    }
}

impl TransactionCommand for crate::commands::HGetCommand {
    fn name(&self) -> &str {
        self.command_name()
    }
    
    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }
    
    fn key(&self) -> Option<String> {
        Some(self.keys()[0].iter().map(|&b| b as char).collect())
    }
}

impl TransactionCommand for crate::commands::HSetCommand {
    fn name(&self) -> &str {
        self.command_name()
    }
    
    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }
    
    fn key(&self) -> Option<String> {
        Some(self.keys()[0].iter().map(|&b| b as char).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    struct MockTransactionExecutor {
        commands: Vec<String>,
        multi_called: bool,
        exec_called: bool,
    }

    impl MockTransactionExecutor {
        fn new() -> Self {
            Self {
                commands: Vec::new(),
                multi_called: false,
                exec_called: false,
            }
        }
    }

    #[async_trait::async_trait]
    impl TransactionExecutor for MockTransactionExecutor {
        async fn multi(&mut self) -> RedisResult<()> {
            self.multi_called = true;
            Ok(())
        }
        
        async fn queue_command(&mut self, command: Box<dyn TransactionCommand>) -> RedisResult<()> {
            self.commands.push(command.name().to_string());
            Ok(())
        }
        
        async fn exec(&mut self) -> RedisResult<Vec<RespValue>> {
            self.exec_called = true;
            let mut results = Vec::new();
            for _ in 0..self.commands.len() {
                results.push(RespValue::SimpleString("OK".to_string()));
            }
            Ok(results)
        }
        
        async fn discard(&mut self) -> RedisResult<()> {
            self.commands.clear();
            self.multi_called = false;
            Ok(())
        }
        
        async fn watch(&mut self, _keys: Vec<String>) -> RedisResult<()> {
            Ok(())
        }
        
        async fn unwatch(&mut self) -> RedisResult<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_transaction_creation() {
        let executor = MockTransactionExecutor::new();
        let transaction = Transaction::new(Arc::new(Mutex::new(executor)));
        
        assert!(transaction.is_empty());
        assert_eq!(transaction.len(), 0);
    }

    #[tokio::test]
    async fn test_transaction_add_commands() {
        let executor = MockTransactionExecutor::new();
        let mut transaction = Transaction::new(Arc::new(Mutex::new(executor)));
        
        transaction.set("key1", "value1");
        transaction.get("key1");
        
        assert_eq!(transaction.len(), 2);
        assert!(!transaction.is_empty());
    }

    #[tokio::test]
    async fn test_transaction_exec() {
        let executor = MockTransactionExecutor::new();
        let mut transaction = Transaction::new(Arc::new(Mutex::new(executor)));
        
        transaction.set("key1", "value1");
        transaction.get("key1");
        
        let results = transaction.exec().await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(transaction.is_empty()); // Commands should be consumed
    }

    #[tokio::test]
    async fn test_transaction_discard() {
        let executor = MockTransactionExecutor::new();
        let mut transaction = Transaction::new(Arc::new(Mutex::new(executor)));
        
        transaction.set("key1", "value1");
        transaction.get("key1");
        assert_eq!(transaction.len(), 2);
        
        transaction.discard().await.unwrap();
        assert!(transaction.is_empty());
    }

    #[tokio::test]
    async fn test_transaction_result() {
        let results = vec![
            RespValue::SimpleString("OK".to_string()),
            RespValue::BulkString("value1".as_bytes().into()),
            RespValue::Integer(42),
        ];
        
        let mut transaction_result = TransactionResult::new(results);
        
        assert_eq!(transaction_result.len(), 3);
        assert!(!transaction_result.is_empty());
        
        let first: String = transaction_result.next().unwrap();
        assert_eq!(first, "OK");
        
        let second: String = transaction_result.get(1).unwrap();
        assert_eq!(second, "value1");
    }
}
