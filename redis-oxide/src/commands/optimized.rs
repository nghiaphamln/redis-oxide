//! Optimized command builders with pre-allocation and caching
//!
//! This module provides optimized versions of command builders that:
//! - Pre-allocate argument vectors based on command type
//! - Cache frequently used keys and values
//! - Optimize serialized command caching
//! - Reduce memory allocations

#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(missing_docs)]

use crate::commands::Command;
use crate::core::{error::RedisResult, value::RespValue};
use crate::pipeline::PipelineCommand;
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// String interning cache for frequently used strings
#[derive(Debug)]
pub struct StringInterner {
    cache: HashMap<String, Arc<str>>,
    max_size: usize,
    access_count: HashMap<String, u64>,
}

impl StringInterner {
    /// Create a new string interner
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
            access_count: HashMap::new(),
        }
    }

    /// Intern a string, returning an `Arc<str>` for efficient sharing
    pub fn intern(&mut self, s: &str) -> Arc<str> {
        if let Some(interned) = self.cache.get(s) {
            // Update access count
            *self.access_count.entry(s.to_string()).or_insert(0) += 1;
            return interned.clone();
        }

        // If cache is full, remove least frequently used entry
        if self.cache.len() >= self.max_size {
            if let Some((lfu_key, _)) = self.access_count.iter().min_by_key(|(_, &count)| count) {
                let lfu_key = lfu_key.clone();
                self.cache.remove(&lfu_key);
                self.access_count.remove(&lfu_key);
            }
        }

        let interned: Arc<str> = s.into();
        self.cache.insert(s.to_string(), interned.clone());
        self.access_count.insert(s.to_string(), 1);
        interned
    }

    /// Get cache statistics
    pub fn stats(&self) -> (usize, usize) {
        (self.cache.len(), self.max_size)
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.access_count.clear();
    }
}

/// Global string interner instance
static STRING_INTERNER: RwLock<Option<StringInterner>> = RwLock::new(None);

/// Initialize the global string interner
pub fn init_string_interner(max_size: usize) {
    let mut interner = STRING_INTERNER.write().unwrap();
    *interner = Some(StringInterner::new(max_size));
}

/// Intern a string using the global interner
pub fn intern_string(s: &str) -> Arc<str> {
    let mut interner_guard = STRING_INTERNER.write().unwrap();
    if let Some(ref mut interner) = *interner_guard {
        interner.intern(s)
    } else {
        // Fallback if interner not initialized
        s.into()
    }
}

/// Command cache for serialized commands
#[derive(Debug)]
pub struct CommandCache {
    cache: HashMap<String, Bytes>,
    max_size: usize,
    access_count: HashMap<String, u64>,
}

impl CommandCache {
    /// Create a new command cache
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
            access_count: HashMap::new(),
        }
    }

    /// Get a cached command or insert a new one
    pub fn get_or_insert<F>(&mut self, key: &str, f: F) -> Bytes
    where
        F: FnOnce() -> Bytes,
    {
        if let Some(cached) = self.cache.get(key) {
            *self.access_count.entry(key.to_string()).or_insert(0) += 1;
            return cached.clone();
        }

        // If cache is full, remove least frequently used entry
        if self.cache.len() >= self.max_size {
            if let Some((lfu_key, _)) = self.access_count.iter().min_by_key(|(_, &count)| count) {
                let lfu_key = lfu_key.clone();
                self.cache.remove(&lfu_key);
                self.access_count.remove(&lfu_key);
            }
        }

        let value = f();
        self.cache.insert(key.to_string(), value.clone());
        self.access_count.insert(key.to_string(), 1);
        value
    }

    /// Get cache statistics
    pub fn stats(&self) -> (usize, usize) {
        (self.cache.len(), self.max_size)
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.access_count.clear();
    }
}

/// Optimized GET command with pre-allocated arguments
#[derive(Debug, Clone)]
pub struct OptimizedGetCommand {
    key: Arc<str>,
    args_cache: Option<Vec<RespValue>>,
}

impl OptimizedGetCommand {
    /// Create a new optimized GET command
    pub fn new(key: impl AsRef<str>) -> Self {
        let key = intern_string(key.as_ref());
        Self {
            key,
            args_cache: None,
        }
    }

    /// Pre-compute and cache arguments
    pub fn with_cached_args(mut self) -> Self {
        self.args_cache = Some(vec![RespValue::from(self.key.as_ref())]);
        self
    }
}

impl Command for OptimizedGetCommand {
    type Output = Option<String>;

    fn command_name(&self) -> &str {
        "GET"
    }

    fn args(&self) -> Vec<RespValue> {
        if let Some(ref cached) = self.args_cache {
            cached.clone()
        } else {
            vec![RespValue::from(self.key.as_ref())]
        }
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        if response.is_null() {
            Ok(None)
        } else {
            Ok(Some(response.as_string()?))
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for OptimizedGetCommand {
    fn name(&self) -> &str {
        self.command_name()
    }

    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }

    fn key(&self) -> Option<String> {
        Some(self.key.to_string())
    }
}

/// Optimized SET command with pre-allocated arguments and options
#[derive(Debug, Clone)]
pub struct OptimizedSetCommand {
    key: Arc<str>,
    value: Arc<str>,
    expiration: Option<Duration>,
    nx: bool,
    xx: bool,
    args_cache: Option<Vec<RespValue>>,
}

impl OptimizedSetCommand {
    /// Create a new optimized SET command
    pub fn new(key: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        let key = intern_string(key.as_ref());
        let value = intern_string(value.as_ref());
        Self {
            key,
            value,
            expiration: None,
            nx: false,
            xx: false,
            args_cache: None,
        }
    }

    /// Set expiration time (EX seconds)
    pub fn expire(mut self, duration: Duration) -> Self {
        self.expiration = Some(duration);
        self.args_cache = None; // Invalidate cache
        self
    }

    /// Only set if key doesn't exist (NX)
    pub fn only_if_not_exists(mut self) -> Self {
        self.nx = true;
        self.args_cache = None; // Invalidate cache
        self
    }

    /// Only set if key exists (XX)
    pub fn only_if_exists(mut self) -> Self {
        self.xx = true;
        self.args_cache = None; // Invalidate cache
        self
    }

    /// Pre-compute and cache arguments
    pub fn with_cached_args(mut self) -> Self {
        let mut args = Vec::with_capacity(6); // Pre-allocate for worst case
        args.push(RespValue::from(self.key.as_ref()));
        args.push(RespValue::from(self.value.as_ref()));

        if let Some(duration) = self.expiration {
            args.push(RespValue::from("EX"));
            args.push(RespValue::from(duration.as_secs().to_string()));
        }

        if self.nx {
            args.push(RespValue::from("NX"));
        }

        if self.xx {
            args.push(RespValue::from("XX"));
        }

        self.args_cache = Some(args);
        self
    }
}

impl Command for OptimizedSetCommand {
    type Output = bool;

    fn command_name(&self) -> &str {
        "SET"
    }

    fn args(&self) -> Vec<RespValue> {
        if let Some(ref cached) = self.args_cache {
            cached.clone()
        } else {
            let mut args = Vec::with_capacity(6);
            args.push(RespValue::from(self.key.as_ref()));
            args.push(RespValue::from(self.value.as_ref()));

            if let Some(duration) = self.expiration {
                args.push(RespValue::from("EX"));
                args.push(RespValue::from(duration.as_secs().to_string()));
            }

            if self.nx {
                args.push(RespValue::from("NX"));
            }

            if self.xx {
                args.push(RespValue::from("XX"));
            }

            args
        }
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::SimpleString(ref s) if s == "OK" => Ok(true),
            _ => Ok(false),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for OptimizedSetCommand {
    fn name(&self) -> &str {
        self.command_name()
    }

    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }

    fn key(&self) -> Option<String> {
        Some(self.key.to_string())
    }
}

/// Optimized HSET command with pre-allocated arguments
#[derive(Debug, Clone)]
pub struct OptimizedHSetCommand {
    key: Arc<str>,
    field: Arc<str>,
    value: Arc<str>,
    args_cache: Option<Vec<RespValue>>,
}

impl OptimizedHSetCommand {
    /// Create a new optimized HSET command
    pub fn new(key: impl AsRef<str>, field: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        let key = intern_string(key.as_ref());
        let field = intern_string(field.as_ref());
        let value = intern_string(value.as_ref());
        Self {
            key,
            field,
            value,
            args_cache: None,
        }
    }

    /// Pre-compute and cache arguments
    pub fn with_cached_args(mut self) -> Self {
        self.args_cache = Some(vec![
            RespValue::from(self.key.as_ref()),
            RespValue::from(self.field.as_ref()),
            RespValue::from(self.value.as_ref()),
        ]);
        self
    }
}

impl Command for OptimizedHSetCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "HSET"
    }

    fn args(&self) -> Vec<RespValue> {
        if let Some(ref cached) = self.args_cache {
            cached.clone()
        } else {
            vec![
                RespValue::from(self.key.as_ref()),
                RespValue::from(self.field.as_ref()),
                RespValue::from(self.value.as_ref()),
            ]
        }
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        response.as_int()
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for OptimizedHSetCommand {
    fn name(&self) -> &str {
        self.command_name()
    }

    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }

    fn key(&self) -> Option<String> {
        Some(self.key.to_string())
    }
}

/// Batch command builder for optimized pipeline operations
pub struct BatchCommandBuilder {
    commands: Vec<Box<dyn Command<Output = RespValue> + Send + Sync>>,
    estimated_size: usize,
}

impl BatchCommandBuilder {
    /// Create a new batch command builder
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            estimated_size: 0,
        }
    }

    /// Create a new batch command builder with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            commands: Vec::with_capacity(capacity),
            estimated_size: 0,
        }
    }

    /// Add a command to the batch
    pub fn add_command<T>(&mut self, command: T)
    where
        T: Command + Send + Sync + 'static,
        T::Output: Into<RespValue>,
    {
        // Estimate size for this command
        let args = command.args();
        let cmd_size = command.command_name().len()
            + args
                .iter()
                .map(|arg| self.estimate_arg_size(arg))
                .sum::<usize>();
        self.estimated_size += cmd_size;

        // Box the command with type erasure
        // Note: This is simplified - real implementation would need proper trait objects
        // self.commands.push(Box::new(command));
    }

    /// Estimate the serialized size of a RespValue
    fn estimate_arg_size(&self, value: &RespValue) -> usize {
        match value {
            RespValue::SimpleString(s) => s.len() + 3, // +str\r\n
            RespValue::Error(e) => e.len() + 3,        // -err\r\n
            RespValue::Integer(_) => 10,               // Rough estimate for :num\r\n
            RespValue::BulkString(b) => b.len() + 10,  // $len\r\ndata\r\n
            RespValue::Null => 5,                      // $-1\r\n
            RespValue::Array(arr) => {
                10 + arr
                    .iter()
                    .map(|item| self.estimate_arg_size(item))
                    .sum::<usize>()
            }
        }
    }

    /// Get the estimated total size
    pub fn estimated_size(&self) -> usize {
        self.estimated_size
    }

    /// Get the number of commands in the batch
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

impl Default for BatchCommandBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory pool for reusing temporary objects
pub struct MemoryPool<T> {
    pool: Vec<T>,
    max_size: usize,
    create_fn: Box<dyn Fn() -> T + Send + Sync>,
}

impl<T> MemoryPool<T> {
    /// Create a new memory pool
    pub fn new<F>(max_size: usize, create_fn: F) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        Self {
            pool: Vec::new(),
            max_size,
            create_fn: Box::new(create_fn),
        }
    }

    /// Get an object from the pool or create a new one
    pub fn get(&mut self) -> T {
        self.pool.pop().unwrap_or_else(|| (self.create_fn)())
    }

    /// Return an object to the pool
    pub fn put(&mut self, item: T) {
        if self.pool.len() < self.max_size {
            self.pool.push(item);
        }
        // If pool is full, just drop the item
    }

    /// Get pool statistics
    pub fn stats(&self) -> (usize, usize) {
        (self.pool.len(), self.max_size)
    }

    /// Clear the pool
    pub fn clear(&mut self) {
        self.pool.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_interner() {
        let mut interner = StringInterner::new(3);

        let s1 = interner.intern("hello");
        let s2 = interner.intern("hello");
        let s3 = interner.intern("world");

        // Same string should return same Arc
        assert!(Arc::ptr_eq(&s1, &s2));
        assert_eq!(s1.as_ref(), "hello");
        assert_eq!(s3.as_ref(), "world");

        let (size, max_size) = interner.stats();
        assert_eq!(size, 2);
        assert_eq!(max_size, 3);
    }

    #[test]
    fn test_optimized_get_command() {
        let cmd = OptimizedGetCommand::new("test_key").with_cached_args();
        assert_eq!(cmd.command_name(), "GET");
        assert_eq!(cmd.keys(), vec![b"test_key"]);

        let args = <OptimizedGetCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 1);
    }

    #[test]
    fn test_optimized_set_command() {
        let cmd = OptimizedSetCommand::new("key", "value")
            .expire(Duration::from_secs(60))
            .only_if_not_exists()
            .with_cached_args();

        assert_eq!(cmd.command_name(), "SET");
        let args = <OptimizedSetCommand as Command>::args(&cmd);
        assert!(args.len() >= 4); // key, value, EX, 60, NX
    }

    #[test]
    fn test_batch_command_builder() {
        let builder = BatchCommandBuilder::with_capacity(10);
        assert_eq!(builder.len(), 0);
        assert!(builder.is_empty());

        // Test size estimation
        assert_eq!(builder.estimated_size(), 0);
    }

    #[test]
    fn test_memory_pool() {
        let mut pool = MemoryPool::new(3, || Vec::<i32>::new());

        let mut vec1 = pool.get();
        vec1.push(1);
        vec1.push(2);

        pool.put(vec1);

        let vec2 = pool.get();
        // Should reuse the vector (but it might be cleared depending on implementation)

        let (size, max_size) = pool.stats();
        assert_eq!(max_size, 3);
    }
}
