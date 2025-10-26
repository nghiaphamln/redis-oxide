#![allow(clippy::items_after_test_module)]
//! Command builders for Redis operations
//!
//! This module provides type-safe command builders for Redis commands.

pub mod hash;
pub mod list;
pub mod optimized;
pub mod set;
pub mod sorted_set;

use crate::core::{error::RedisResult, value::RespValue};
use crate::pipeline::PipelineCommand;
use std::time::Duration;

// Re-export hash commands
pub use hash::{
    HDelCommand, HExistsCommand, HGetAllCommand, HGetCommand, HLenCommand, HMGetCommand,
    HMSetCommand, HSetCommand,
};

// Re-export list commands
pub use list::{
    LIndexCommand, LLenCommand, LPopCommand, LPushCommand, LRangeCommand, LSetCommand, RPopCommand,
    RPushCommand,
};

// Re-export set commands
pub use set::{
    SAddCommand, SCardCommand, SIsMemberCommand, SMembersCommand, SPopCommand, SRandMemberCommand,
    SRemCommand,
};

// Re-export sorted set commands
pub use sorted_set::{
    ZAddCommand, ZCardCommand, ZRangeCommand, ZRankCommand, ZRemCommand, ZRevRankCommand,
    ZScoreCommand,
};

/// Trait for commands that can be executed
pub trait Command {
    /// The return type of the command
    type Output;

    /// Get the command name
    fn command_name(&self) -> &str;

    /// Get the command arguments
    fn args(&self) -> Vec<RespValue>;

    /// Parse the response into the output type
    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output>;

    /// Get the key(s) involved in this command (for cluster routing)
    fn keys(&self) -> Vec<&[u8]>;
}

/// GET command builder
pub struct GetCommand {
    key: String,
}

impl GetCommand {
    /// Create a new GET command
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }
}

impl Command for GetCommand {
    type Output = Option<String>;

    fn command_name(&self) -> &str {
        "GET"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![RespValue::from(self.key.as_str())]
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

/// SET command builder
pub struct SetCommand {
    key: String,
    value: String,
    expiration: Option<Duration>,
    nx: bool,
    xx: bool,
}

impl SetCommand {
    /// Create a new SET command
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            expiration: None,
            nx: false,
            xx: false,
        }
    }

    /// Set expiration time (EX seconds)
    pub fn expire(mut self, duration: Duration) -> Self {
        self.expiration = Some(duration);
        self
    }

    /// Only set if key doesn't exist (NX)
    pub fn only_if_not_exists(mut self) -> Self {
        self.nx = true;
        self
    }

    /// Only set if key exists (XX)
    pub fn only_if_exists(mut self) -> Self {
        self.xx = true;
        self
    }
}

impl Command for SetCommand {
    type Output = bool;

    fn command_name(&self) -> &str {
        "SET"
    }

    fn args(&self) -> Vec<RespValue> {
        let mut args = vec![
            RespValue::from(self.key.as_str()),
            RespValue::from(self.value.as_str()),
        ];

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

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::SimpleString(ref s) if s == "OK" => Ok(true),
            // NX or XX condition not met
            _ => Ok(false),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

/// DEL command builder
pub struct DelCommand {
    keys: Vec<String>,
}

impl DelCommand {
    /// Create a new DEL command
    pub fn new(keys: Vec<String>) -> Self {
        Self { keys }
    }
}

impl Command for DelCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "DEL"
    }

    fn args(&self) -> Vec<RespValue> {
        self.keys
            .iter()
            .map(|k| RespValue::from(k.as_str()))
            .collect()
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        response.as_int()
    }

    fn keys(&self) -> Vec<&[u8]> {
        self.keys.iter().map(String::as_bytes).collect()
    }
}

/// EXISTS command builder
pub struct ExistsCommand {
    keys: Vec<String>,
}

impl ExistsCommand {
    /// Create a new EXISTS command
    pub fn new(keys: Vec<String>) -> Self {
        Self { keys }
    }
}

impl Command for ExistsCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "EXISTS"
    }

    fn args(&self) -> Vec<RespValue> {
        self.keys
            .iter()
            .map(|k| RespValue::from(k.as_str()))
            .collect()
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        response.as_int()
    }

    fn keys(&self) -> Vec<&[u8]> {
        self.keys.iter().map(String::as_bytes).collect()
    }
}

/// EXPIRE command builder
pub struct ExpireCommand {
    key: String,
    seconds: i64,
}

impl ExpireCommand {
    /// Create a new EXPIRE command
    pub fn new(key: impl Into<String>, duration: Duration) -> Self {
        #[allow(clippy::cast_possible_wrap)]
        Self {
            key: key.into(),
            seconds: duration.as_secs() as i64,
        }
    }
}

impl Command for ExpireCommand {
    type Output = bool;

    fn command_name(&self) -> &str {
        "EXPIRE"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![
            RespValue::from(self.key.as_str()),
            RespValue::from(self.seconds.to_string()),
        ]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        Ok(response.as_int()? == 1)
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

/// TTL command builder
pub struct TtlCommand {
    key: String,
}

impl TtlCommand {
    /// Create a new TTL command
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }
}

impl Command for TtlCommand {
    type Output = Option<i64>;

    fn command_name(&self) -> &str {
        "TTL"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![RespValue::from(self.key.as_str())]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        let ttl = response.as_int()?;
        if ttl < 0 {
            Ok(None) // -2 = key doesn't exist, -1 = no expiration
        } else {
            Ok(Some(ttl))
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

/// INCR command builder
pub struct IncrCommand {
    key: String,
}

impl IncrCommand {
    /// Create a new INCR command
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }
}

impl Command for IncrCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "INCR"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![RespValue::from(self.key.as_str())]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        response.as_int()
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

/// DECR command builder
pub struct DecrCommand {
    key: String,
}

impl DecrCommand {
    /// Create a new DECR command
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }
}

impl Command for DecrCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "DECR"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![RespValue::from(self.key.as_str())]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        response.as_int()
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

/// INCRBY command builder
pub struct IncrByCommand {
    key: String,
    increment: i64,
}

impl IncrByCommand {
    /// Create a new INCRBY command
    pub fn new(key: impl Into<String>, increment: i64) -> Self {
        Self {
            key: key.into(),
            increment,
        }
    }
}

impl Command for IncrByCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "INCRBY"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![
            RespValue::from(self.key.as_str()),
            RespValue::from(self.increment.to_string()),
        ]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        response.as_int()
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

/// DECRBY command builder
pub struct DecrByCommand {
    key: String,
    decrement: i64,
}

impl DecrByCommand {
    /// Create a new DECRBY command
    pub fn new(key: impl Into<String>, decrement: i64) -> Self {
        Self {
            key: key.into(),
            decrement,
        }
    }
}

impl Command for DecrByCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "DECRBY"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![
            RespValue::from(self.key.as_str()),
            RespValue::from(self.decrement.to_string()),
        ]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        response.as_int()
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_command() {
        let cmd = GetCommand::new("mykey");
        assert_eq!(cmd.command_name(), "GET");
        assert_eq!(cmd.keys(), vec![b"mykey"]);
    }

    #[test]
    fn test_set_command_basic() {
        let cmd = SetCommand::new("key", "value");
        assert_eq!(cmd.command_name(), "SET");
        let args = <SetCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_set_command_with_expiration() {
        let cmd = SetCommand::new("key", "value").expire(Duration::from_secs(60));
        let args = <SetCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 4); // key, value, EX, 60
    }

    #[test]
    fn test_set_command_nx() {
        let cmd = SetCommand::new("key", "value").only_if_not_exists();
        let args = <SetCommand as Command>::args(&cmd);
        assert!(args.len() >= 3); // key, value, NX
    }

    #[test]
    fn test_del_command() {
        let cmd = DelCommand::new(vec!["key1".to_string(), "key2".to_string()]);
        assert_eq!(cmd.command_name(), "DEL");
        assert_eq!(cmd.keys().len(), 2);
    }

    #[test]
    fn test_incr_command() {
        let cmd = IncrCommand::new("counter");
        assert_eq!(cmd.command_name(), "INCR");
        assert_eq!(cmd.keys(), vec![b"counter"]);
    }
}

// Implement PipelineCommand for all command types
impl PipelineCommand for GetCommand {
    fn name(&self) -> &str {
        self.command_name()
    }

    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }

    fn key(&self) -> Option<String> {
        Some(self.key.clone())
    }
}

impl PipelineCommand for SetCommand {
    fn name(&self) -> &str {
        self.command_name()
    }

    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }

    fn key(&self) -> Option<String> {
        Some(self.key.clone())
    }
}

impl PipelineCommand for DelCommand {
    fn name(&self) -> &str {
        self.command_name()
    }

    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }

    fn key(&self) -> Option<String> {
        self.keys.first().cloned()
    }
}

impl PipelineCommand for IncrCommand {
    fn name(&self) -> &str {
        self.command_name()
    }

    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }

    fn key(&self) -> Option<String> {
        Some(self.key.clone())
    }
}

impl PipelineCommand for DecrCommand {
    fn name(&self) -> &str {
        self.command_name()
    }

    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }

    fn key(&self) -> Option<String> {
        Some(self.key.clone())
    }
}

impl PipelineCommand for IncrByCommand {
    fn name(&self) -> &str {
        self.command_name()
    }

    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }

    fn key(&self) -> Option<String> {
        Some(self.key.clone())
    }
}

impl PipelineCommand for DecrByCommand {
    fn name(&self) -> &str {
        self.command_name()
    }

    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }

    fn key(&self) -> Option<String> {
        Some(self.key.clone())
    }
}

impl PipelineCommand for ExistsCommand {
    fn name(&self) -> &str {
        self.command_name()
    }

    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }

    fn key(&self) -> Option<String> {
        self.keys.first().cloned()
    }
}

impl PipelineCommand for ExpireCommand {
    fn name(&self) -> &str {
        self.command_name()
    }

    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }

    fn key(&self) -> Option<String> {
        Some(self.key.clone())
    }
}

impl PipelineCommand for TtlCommand {
    fn name(&self) -> &str {
        self.command_name()
    }

    fn args(&self) -> Vec<RespValue> {
        <Self as Command>::args(self)
    }

    fn key(&self) -> Option<String> {
        Some(self.key.clone())
    }
}
