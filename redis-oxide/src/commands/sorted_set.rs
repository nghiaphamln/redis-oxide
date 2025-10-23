//! Command builders for Redis Sorted Set operations

use crate::core::{
    error::{RedisError, RedisResult},
    value::RespValue,
};
use crate::commands::Command;
use crate::pipeline::PipelineCommand;
use std::collections::HashMap;
use std::convert::TryFrom;

/// Represents the `ZADD` command.
#[derive(Debug, Clone)]
pub struct ZAddCommand {
    key: String,
    members: HashMap<String, f64>,
}

impl ZAddCommand {
    /// Create a new `ZADD` command.
    #[must_use]
    pub fn new(key: impl Into<String>, members: HashMap<String, f64>) -> Self {
        Self {
            key: key.into(),
            members,
        }
    }
}

impl Command for ZAddCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "ZADD"
    }

    fn args(&self) -> Vec<RespValue> {
        let mut args = vec![RespValue::from(self.key.clone())];
        for (member, score) in &self.members {
            args.push(RespValue::from(score.to_string()));
            args.push(RespValue::from(member.clone()));
        }
        args
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        response.as_int()
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for ZAddCommand {
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

/// Represents the `ZREM` command.
#[derive(Debug, Clone)]
pub struct ZRemCommand {
    key: String,
    members: Vec<String>,
}

impl ZRemCommand {
    /// Create a new `ZREM` command.
    #[must_use]
    pub fn new(key: impl Into<String>, members: Vec<String>) -> Self {
        Self {
            key: key.into(),
            members,
        }
    }
}

impl Command for ZRemCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "ZREM"
    }

    fn args(&self) -> Vec<RespValue> {
        let mut args = vec![RespValue::from(self.key.clone())];
        for member in &self.members {
            args.push(RespValue::from(member.clone()));
        }
        args
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        response.as_int()
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for ZRemCommand {
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

/// Represents the `ZRANGE` command.
#[derive(Debug, Clone)]
pub struct ZRangeCommand {
    key: String,
    start: i64,
    stop: i64,
}

impl ZRangeCommand {
    /// Create a new `ZRANGE` command.
    #[must_use]
    pub fn new(key: impl Into<String>, start: i64, stop: i64) -> Self {
        Self {
            key: key.into(),
            start,
            stop,
        }
    }
}

impl Command for ZRangeCommand {
    type Output = Vec<String>;

    fn command_name(&self) -> &str {
        "ZRANGE"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![
            RespValue::from(self.key.clone()),
            RespValue::from(self.start),
            RespValue::from(self.stop),
        ]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::Array(items) => {
                let mut result = Vec::new();
                for item in items {
                    match item {
                        RespValue::BulkString(b) => {
                            let s = String::from_utf8(b.to_vec())
                                .map_err(|e| RedisError::Type(format!("Invalid UTF-8: {e}")))?;
                            result.push(s);
                        }
                        RespValue::Null => {
                            // Skip null values
                        }
                        _ => return Err(RedisError::Type(format!(
                            "Unexpected item type in ZRANGE response: {:?}",
                            item
                        ))),
                    }
                }
                Ok(result)
            }
            _ => Err(RedisError::Type(format!(
                "Unexpected response type for ZRANGE: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for ZRangeCommand {
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

/// Represents the `ZSCORE` command.
#[derive(Debug, Clone)]
pub struct ZScoreCommand {
    key: String,
    member: String,
}

impl ZScoreCommand {
    /// Create a new `ZSCORE` command.
    #[must_use]
    pub fn new(key: impl Into<String>, member: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            member: member.into(),
        }
    }
}

impl Command for ZScoreCommand {
    type Output = Option<f64>;

    fn command_name(&self) -> &str {
        "ZSCORE"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![
            RespValue::from(self.key.clone()),
            RespValue::from(self.member.clone()),
        ]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::BulkString(b) => {
                let s = String::from_utf8(b.to_vec())
                    .map_err(|e| RedisError::Type(format!("Invalid UTF-8: {e}")))?;
                let score = s.parse::<f64>()
                    .map_err(|e| RedisError::Type(format!("Invalid float: {e}")))?;
                Ok(Some(score))
            }
            RespValue::Null => Ok(None),
            _ => Err(RedisError::Type(format!(
                "Unexpected response type for ZSCORE: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for ZScoreCommand {
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

/// Represents the `ZCARD` command.
#[derive(Debug, Clone)]
pub struct ZCardCommand {
    key: String,
}

impl ZCardCommand {
    /// Create a new `ZCARD` command.
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
        }
    }
}

impl Command for ZCardCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "ZCARD"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![RespValue::from(self.key.clone())]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        response.as_int()
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for ZCardCommand {
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

/// Represents the `ZRANK` command.
#[derive(Debug, Clone)]
pub struct ZRankCommand {
    key: String,
    member: String,
}

impl ZRankCommand {
    /// Create a new `ZRANK` command.
    #[must_use]
    pub fn new(key: impl Into<String>, member: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            member: member.into(),
        }
    }
}

impl Command for ZRankCommand {
    type Output = Option<i64>;

    fn command_name(&self) -> &str {
        "ZRANK"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![
            RespValue::from(self.key.clone()),
            RespValue::from(self.member.clone()),
        ]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::Integer(rank) => Ok(Some(rank)),
            RespValue::Null => Ok(None),
            _ => Err(RedisError::Type(format!(
                "Unexpected response type for ZRANK: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for ZRankCommand {
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

/// Represents the `ZREVRANK` command.
#[derive(Debug, Clone)]
pub struct ZRevRankCommand {
    key: String,
    member: String,
}

impl ZRevRankCommand {
    /// Create a new `ZREVRANK` command.
    #[must_use]
    pub fn new(key: impl Into<String>, member: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            member: member.into(),
        }
    }
}

impl Command for ZRevRankCommand {
    type Output = Option<i64>;

    fn command_name(&self) -> &str {
        "ZREVRANK"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![
            RespValue::from(self.key.clone()),
            RespValue::from(self.member.clone()),
        ]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::Integer(rank) => Ok(Some(rank)),
            RespValue::Null => Ok(None),
            _ => Err(RedisError::Type(format!(
                "Unexpected response type for ZREVRANK: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for ZRevRankCommand {
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