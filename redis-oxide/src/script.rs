//! Lua scripting support for Redis
//!
//! This module provides functionality for executing Lua scripts on Redis servers
//! using EVAL and EVALSHA commands. Scripts are automatically cached and managed
//! for optimal performance.
//!
//! # Examples
//!
//! ## Basic Script Execution
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ConnectionConfig::new("redis://localhost:6379");
//! let client = Client::connect(config).await?;
//!
//! // Execute a simple Lua script
//! let script = "return redis.call('GET', KEYS[1])";
//! let result: Option<String> = client.eval(script, vec!["mykey".to_string()], vec![]).await?;
//! println!("Result: {:?}", result);
//! # Ok(())
//! # }
//! ```
//!
//! ## Script with Arguments
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ConnectionConfig::new("redis://localhost:6379");
//! let client = Client::connect(config).await?;
//!
//! // Script that increments a counter by a given amount
//! let script = r#"
//!     local current = redis.call('GET', KEYS[1]) or 0
//!     local increment = tonumber(ARGV[1])
//!     local new_value = tonumber(current) + increment
//!     redis.call('SET', KEYS[1], new_value)
//!     return new_value
//! "#;
//!
//! let result: i64 = client.eval(
//!     script,
//!     vec!["counter".to_string()],
//!     vec!["5".to_string()]
//! ).await?;
//! println!("New counter value: {}", result);
//! # Ok(())
//! # }
//! ```
//!
//! ## Using Script Manager for Caching
//!
//! ```no_run
//! use redis_oxide::{Client, ConnectionConfig, Script};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ConnectionConfig::new("redis://localhost:6379");
//! let client = Client::connect(config).await?;
//!
//! // Create a reusable script
//! let script = Script::new(r#"
//!     local key = KEYS[1]
//!     local value = ARGV[1]
//!     redis.call('SET', key, value)
//!     return redis.call('GET', key)
//! "#);
//!
//! // Execute the script (automatically uses EVALSHA if cached)
//! let result: String = script.execute(
//!     &client,
//!     vec!["mykey".to_string()],
//!     vec!["myvalue".to_string()]
//! ).await?;
//! println!("Result: {}", result);
//! # Ok(())
//! # }
//! ```

use crate::core::{
    error::{RedisError, RedisResult},
    value::RespValue,
};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A Lua script that can be executed on Redis
#[derive(Debug, Clone)]
pub struct Script {
    /// The Lua script source code
    source: String,
    /// SHA1 hash of the script (for EVALSHA)
    sha: String,
}

impl Script {
    /// Create a new script from Lua source code
    ///
    /// The script is automatically hashed for use with EVALSHA.
    ///
    /// # Examples
    ///
    /// ```
    /// use redis_oxide::Script;
    ///
    /// let script = Script::new("return redis.call('GET', KEYS[1])");
    /// println!("Script SHA: {}", script.sha());
    /// ```
    pub fn new(source: impl Into<String>) -> Self {
        let source = source.into();
        let sha = calculate_sha1(&source);

        Self { source, sha }
    }

    /// Get the SHA1 hash of the script
    #[must_use]
    pub fn sha(&self) -> &str {
        &self.sha
    }

    /// Get the source code of the script
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Execute the script on the given client
    ///
    /// This method will first try to use EVALSHA (if the script is cached on the server),
    /// and fall back to EVAL if the script is not cached.
    ///
    /// # Arguments
    ///
    /// * `client` - The Redis client to execute the script on
    /// * `keys` - List of Redis keys that the script will access (KEYS array in Lua)
    /// * `args` - List of arguments to pass to the script (ARGV array in Lua)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use redis_oxide::{Client, ConnectionConfig, Script};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ConnectionConfig::new("redis://localhost:6379");
    /// # let client = Client::connect(config).await?;
    /// let script = Script::new("return KEYS[1] .. ':' .. ARGV[1]");
    ///
    /// let result: String = script.execute(
    ///     &client,
    ///     vec!["user".to_string()],
    ///     vec!["123".to_string()]
    /// ).await?;
    ///
    /// assert_eq!(result, "user:123");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute<T>(
        &self,
        client: &crate::Client,
        keys: Vec<String>,
        args: Vec<String>,
    ) -> RedisResult<T>
    where
        T: TryFrom<RespValue>,
        T::Error: Into<RedisError>,
    {
        // First try EVALSHA
        match client.evalsha(&self.sha, keys.clone(), args.clone()).await {
            Ok(result) => Ok(result),
            Err(RedisError::Protocol(msg)) if msg.contains("NOSCRIPT") => {
                // Script not cached, use EVAL
                client.eval(&self.source, keys, args).await
            }
            Err(e) => Err(e),
        }
    }

    /// Load the script into Redis cache
    ///
    /// This sends the script to Redis using SCRIPT LOAD, which caches it
    /// for future EVALSHA calls.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use redis_oxide::{Client, ConnectionConfig, Script};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ConnectionConfig::new("redis://localhost:6379");
    /// # let client = Client::connect(config).await?;
    /// let script = Script::new("return 'Hello, World!'");
    ///
    /// // Preload the script
    /// let sha = script.load(&client).await?;
    /// println!("Script loaded with SHA: {}", sha);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn load(&self, client: &crate::Client) -> RedisResult<String> {
        client.script_load(&self.source).await
    }
}

/// Script manager for caching and managing multiple scripts
#[derive(Debug)]
pub struct ScriptManager {
    scripts: Arc<RwLock<HashMap<String, Script>>>,
}

impl ScriptManager {
    /// Create a new script manager
    #[must_use]
    pub fn new() -> Self {
        Self {
            scripts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a script with the manager
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use redis_oxide::{Script, ScriptManager};
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut manager = ScriptManager::new();
    /// let script = Script::new("return 'Hello'");
    ///
    /// manager.register("greeting", script).await;
    /// # }
    /// ```
    pub async fn register(&self, name: impl Into<String>, script: Script) {
        let mut scripts = self.scripts.write().await;
        scripts.insert(name.into(), script);
    }

    /// Get a script by name
    ///
    /// # Examples
    ///
    /// ```
    /// # use redis_oxide::{Script, ScriptManager};
    /// # #[tokio::main]
    /// # async fn main() {
    /// let manager = ScriptManager::new();
    /// let script = Script::new("return 'Hello'");
    /// manager.register("greeting", script).await;
    ///
    /// if let Some(script) = manager.get("greeting").await {
    ///     println!("Found script with SHA: {}", script.sha());
    /// }
    /// # }
    /// ```
    pub async fn get(&self, name: &str) -> Option<Script> {
        let scripts = self.scripts.read().await;
        scripts.get(name).cloned()
    }

    /// Execute a script by name
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use redis_oxide::{Client, ConnectionConfig, Script, ScriptManager};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ConnectionConfig::new("redis://localhost:6379");
    /// # let client = Client::connect(config).await?;
    /// let manager = ScriptManager::new();
    /// let script = Script::new("return KEYS[1]");
    /// manager.register("get_key", script).await;
    ///
    /// let result: String = manager.execute(
    ///     "get_key",
    ///     &client,
    ///     vec!["mykey".to_string()],
    ///     vec![]
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute<T>(
        &self,
        name: &str,
        client: &crate::Client,
        keys: Vec<String>,
        args: Vec<String>,
    ) -> RedisResult<T>
    where
        T: TryFrom<RespValue>,
        T::Error: Into<RedisError>,
    {
        let script = self
            .get(name)
            .await
            .ok_or_else(|| RedisError::Protocol(format!("Script '{}' not found", name)))?;

        script.execute(client, keys, args).await
    }

    /// Load all registered scripts into Redis cache
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use redis_oxide::{Client, ConnectionConfig, Script, ScriptManager};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = ConnectionConfig::new("redis://localhost:6379");
    /// # let client = Client::connect(config).await?;
    /// let manager = ScriptManager::new();
    ///
    /// // Register some scripts
    /// manager.register("script1", Script::new("return 1")).await;
    /// manager.register("script2", Script::new("return 2")).await;
    ///
    /// // Load all scripts at once
    /// let results = manager.load_all(&client).await?;
    /// println!("Loaded {} scripts", results.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn load_all(&self, client: &crate::Client) -> RedisResult<HashMap<String, String>> {
        let scripts = self.scripts.read().await;
        let mut results = HashMap::new();

        for (name, script) in scripts.iter() {
            let sha = script.load(client).await?;
            results.insert(name.clone(), sha);
        }

        Ok(results)
    }

    /// Get the number of registered scripts
    #[must_use]
    pub async fn len(&self) -> usize {
        let scripts = self.scripts.read().await;
        scripts.len()
    }

    /// Check if the manager has any scripts
    #[must_use]
    pub async fn is_empty(&self) -> bool {
        let scripts = self.scripts.read().await;
        scripts.is_empty()
    }

    /// List all registered script names
    pub async fn list_scripts(&self) -> Vec<String> {
        let scripts = self.scripts.read().await;
        scripts.keys().cloned().collect()
    }

    /// Remove a script from the manager
    pub async fn remove(&self, name: &str) -> Option<Script> {
        let mut scripts = self.scripts.write().await;
        scripts.remove(name)
    }

    /// Clear all scripts from the manager
    pub async fn clear(&self) {
        let mut scripts = self.scripts.write().await;
        scripts.clear();
    }
}

impl Default for ScriptManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate SHA1 hash of a string
fn calculate_sha1(input: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Common Lua script patterns
pub mod patterns {
    use super::Script;

    /// Atomic increment with expiration
    ///
    /// # Arguments
    /// - KEYS[1]: The key to increment
    /// - ARGV\[1\]: Increment amount
    /// - ARGV\[2\]: Expiration time in seconds
    pub fn atomic_increment_with_expiration() -> Script {
        Script::new(
            r"
            local key = KEYS[1]
            local increment = tonumber(ARGV[1])
            local expiration = tonumber(ARGV[2])
            
            local current = redis.call('GET', key)
            local new_value
            
            if current == false then
                new_value = increment
            else
                new_value = tonumber(current) + increment
            end
            
            redis.call('SET', key, new_value)
            redis.call('EXPIRE', key, expiration)
            
            return new_value
        ",
        )
    }

    /// Conditional set (SET if value matches)
    ///
    /// # Arguments
    /// - KEYS\[1\]: The key to set
    /// - ARGV\[1\]: Expected current value
    /// - ARGV\[2\]: New value to set
    pub fn conditional_set() -> Script {
        Script::new(
            r"
            local key = KEYS[1]
            local expected = ARGV[1]
            local new_value = ARGV[2]
            
            local current = redis.call('GET', key)
            
            if current == expected then
                redis.call('SET', key, new_value)
                return 1
            else
                return 0
            end
        ",
        )
    }

    /// Rate limiting with sliding window
    ///
    /// # Arguments
    /// - KEYS\[1\]: The rate limit key
    /// - ARGV\[1\]: Window size in seconds
    /// - ARGV\[2\]: Maximum requests per window
    pub fn sliding_window_rate_limit() -> Script {
        Script::new(
            r#"
            local key = KEYS[1]
            local window = tonumber(ARGV[1])
            local limit = tonumber(ARGV[2])
            local now = redis.call('TIME')[1]
            
            -- Remove old entries
            redis.call('ZREMRANGEBYSCORE', key, 0, now - window)
            
            -- Count current entries
            local current = redis.call('ZCARD', key)
            
            if current < limit then
                -- Add current request
                redis.call('ZADD', key, now, now)
                redis.call('EXPIRE', key, window)
                return { 1, limit - current - 1 }
            else
                return { 0, 0 }
            end
        "#,
        )
    }

    /// Distributed lock with expiration
    ///
    /// # Arguments
    /// - KEYS\[1\]: The lock key
    /// - ARGV\[1\]: Lock identifier (unique per client)
    /// - ARGV\[2\]: Lock expiration in seconds
    pub fn distributed_lock() -> Script {
        Script::new(
            r#"
            local key = KEYS[1]
            local identifier = ARGV[1]
            local expiration = tonumber(ARGV[2])
            
            if redis.call('SET', key, identifier, 'NX', 'EX', expiration) then
                return 1
            else
                return 0
            end
        "#,
        )
    }

    /// Release distributed lock
    ///
    /// # Arguments
    /// - KEYS\[1\]: The lock key
    /// - ARGV\[1\]: Lock identifier (must match)
    pub fn release_lock() -> Script {
        Script::new(
            r#"
            local key = KEYS[1]
            local identifier = ARGV[1]
            
            if redis.call('GET', key) == identifier then
                return redis.call('DEL', key)
            else
                return 0
            end
        "#,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_creation() {
        let script = Script::new("return 'hello'");
        assert_eq!(script.source(), "return 'hello'");
        assert!(!script.sha().is_empty());
        assert_eq!(script.sha().len(), 40); // SHA1 is 40 hex characters
    }

    #[test]
    fn test_sha1_calculation() {
        let sha = calculate_sha1("hello world");
        assert_eq!(sha, "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed");
    }

    #[test]
    fn test_script_sha_consistency() {
        let script1 = Script::new("return 1");
        let script2 = Script::new("return 1");
        assert_eq!(script1.sha(), script2.sha());
    }

    #[test]
    fn test_script_sha_uniqueness() {
        let script1 = Script::new("return 1");
        let script2 = Script::new("return 2");
        assert_ne!(script1.sha(), script2.sha());
    }

    #[tokio::test]
    async fn test_script_manager_creation() {
        let manager = ScriptManager::new();
        assert!(manager.is_empty().await);
        assert_eq!(manager.len().await, 0);
    }

    #[tokio::test]
    async fn test_script_manager_register_and_get() {
        let manager = ScriptManager::new();
        let script = Script::new("return 'test'");
        let sha = script.sha().to_string();

        manager.register("test_script", script).await;

        assert!(!manager.is_empty().await);
        assert_eq!(manager.len().await, 1);

        let retrieved = manager.get("test_script").await.unwrap();
        assert_eq!(retrieved.sha(), sha);
        assert_eq!(retrieved.source(), "return 'test'");
    }

    #[tokio::test]
    async fn test_script_manager_remove() {
        let manager = ScriptManager::new();
        let script = Script::new("return 'test'");

        manager.register("test_script", script).await;
        assert_eq!(manager.len().await, 1);

        let removed = manager.remove("test_script").await;
        assert!(removed.is_some());
        assert_eq!(manager.len().await, 0);

        let not_found = manager.remove("nonexistent").await;
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_script_manager_clear() {
        let manager = ScriptManager::new();

        manager.register("script1", Script::new("return 1")).await;
        manager.register("script2", Script::new("return 2")).await;
        assert_eq!(manager.len().await, 2);

        manager.clear().await;
        assert_eq!(manager.len().await, 0);
        assert!(manager.is_empty().await);
    }

    #[tokio::test]
    async fn test_script_manager_list_scripts() {
        let manager = ScriptManager::new();

        manager
            .register("script_a", Script::new("return 'a'"))
            .await;
        manager
            .register("script_b", Script::new("return 'b'"))
            .await;

        let mut scripts = manager.list_scripts().await;
        scripts.sort();

        assert_eq!(scripts, vec!["script_a", "script_b"]);
    }

    #[test]
    fn test_pattern_scripts() {
        // Test that pattern scripts can be created without panicking
        let _increment = patterns::atomic_increment_with_expiration();
        let _conditional = patterns::conditional_set();
        let _rate_limit = patterns::sliding_window_rate_limit();
        let _lock = patterns::distributed_lock();
        let _unlock = patterns::release_lock();
    }
}
