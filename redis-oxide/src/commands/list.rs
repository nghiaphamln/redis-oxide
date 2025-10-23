//! Command builders for Redis List operations

use crate::core::{
    error::{RedisError, RedisResult},
    value::RespValue,
};
use crate::commands::Command;
use crate::pipeline::PipelineCommand;
use std::convert::TryFrom;

/// Represents the `LPUSH` command.
#[derive(Debug, Clone)]
pub struct LPushCommand {
    key: String,
    values: Vec<String>,
}

impl LPushCommand {
    /// Create a new `LPUSH` command.
    #[must_use]
    pub fn new(key: impl Into<String>, values: Vec<String>) -> Self {
        Self {
            key: key.into(),
            values,
        }
    }
}

impl Command for LPushCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "LPUSH"
    }

    fn args(&self) -> Vec<RespValue> {
        let mut args = vec![RespValue::from(self.key.clone())];
        for value in &self.values {
            args.push(RespValue::from(value.clone()));
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

impl PipelineCommand for LPushCommand {
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

/// Represents the `RPUSH` command.
#[derive(Debug, Clone)]
pub struct RPushCommand {
    key: String,
    values: Vec<String>,
}

impl RPushCommand {
    /// Create a new `RPUSH` command.
    #[must_use]
    pub fn new(key: impl Into<String>, values: Vec<String>) -> Self {
        Self {
            key: key.into(),
            values,
        }
    }
}

impl Command for RPushCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "RPUSH"
    }

    fn args(&self) -> Vec<RespValue> {
        let mut args = vec![RespValue::from(self.key.clone())];
        for value in &self.values {
            args.push(RespValue::from(value.clone()));
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

impl PipelineCommand for RPushCommand {
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

/// Represents the `LPOP` command.
#[derive(Debug, Clone)]
pub struct LPopCommand {
    key: String,
}

impl LPopCommand {
    /// Create a new `LPOP` command.
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
        }
    }
}

impl Command for LPopCommand {
    type Output = Option<String>;

    fn command_name(&self) -> &str {
        "LPOP"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![RespValue::from(self.key.clone())]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::BulkString(b) => {
                String::from_utf8(b.to_vec())
                    .map(Some)
                    .map_err(|e| RedisError::Type(format!("Invalid UTF-8: {e}")))
            }
            RespValue::Null => Ok(None),
            _ => Err(RedisError::Type(format!(
                "Unexpected response type for LPOP: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for LPopCommand {
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

/// Represents the `RPOP` command.
#[derive(Debug, Clone)]
pub struct RPopCommand {
    key: String,
}

impl RPopCommand {
    /// Create a new `RPOP` command.
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
        }
    }
}

impl Command for RPopCommand {
    type Output = Option<String>;

    fn command_name(&self) -> &str {
        "RPOP"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![RespValue::from(self.key.clone())]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::BulkString(b) => {
                String::from_utf8(b.to_vec())
                    .map(Some)
                    .map_err(|e| RedisError::Type(format!("Invalid UTF-8: {e}")))
            }
            RespValue::Null => Ok(None),
            _ => Err(RedisError::Type(format!(
                "Unexpected response type for RPOP: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for RPopCommand {
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

/// Represents the `LRANGE` command.
#[derive(Debug, Clone)]
pub struct LRangeCommand {
    key: String,
    start: i64,
    stop: i64,
}

impl LRangeCommand {
    /// Create a new `LRANGE` command.
    #[must_use]
    pub fn new(key: impl Into<String>, start: i64, stop: i64) -> Self {
        Self {
            key: key.into(),
            start,
            stop,
        }
    }
}

impl Command for LRangeCommand {
    type Output = Vec<String>;

    fn command_name(&self) -> &str {
        "LRANGE"
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
                            "Unexpected item type in LRANGE response: {:?}",
                            item
                        ))),
                    }
                }
                Ok(result)
            }
            _ => Err(RedisError::Type(format!(
                "Unexpected response type for LRANGE: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for LRangeCommand {
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

/// Represents the `LLEN` command.
#[derive(Debug, Clone)]
pub struct LLenCommand {
    key: String,
}

impl LLenCommand {
    /// Create a new `LLEN` command.
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
        }
    }
}

impl Command for LLenCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "LLEN"
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

impl PipelineCommand for LLenCommand {
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

/// Represents the `LINDEX` command.
#[derive(Debug, Clone)]
pub struct LIndexCommand {
    key: String,
    index: i64,
}

impl LIndexCommand {
    /// Create a new `LINDEX` command.
    #[must_use]
    pub fn new(key: impl Into<String>, index: i64) -> Self {
        Self {
            key: key.into(),
            index,
        }
    }
}

impl Command for LIndexCommand {
    type Output = Option<String>;

    fn command_name(&self) -> &str {
        "LINDEX"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![
            RespValue::from(self.key.clone()),
            RespValue::from(self.index),
        ]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::BulkString(b) => {
                String::from_utf8(b.to_vec())
                    .map(Some)
                    .map_err(|e| RedisError::Type(format!("Invalid UTF-8: {e}")))
            }
            RespValue::Null => Ok(None),
            _ => Err(RedisError::Type(format!(
                "Unexpected response type for LINDEX: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for LIndexCommand {
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

/// Represents the `LSET` command.
#[derive(Debug, Clone)]
pub struct LSetCommand {
    key: String,
    index: i64,
    value: String,
}

impl LSetCommand {
    /// Create a new `LSET` command.
    #[must_use]
    pub fn new(key: impl Into<String>, index: i64, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            index,
            value: value.into(),
        }
    }
}

impl Command for LSetCommand {
    type Output = String;

    fn command_name(&self) -> &str {
        "LSET"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![
            RespValue::from(self.key.clone()),
            RespValue::from(self.index),
            RespValue::from(self.value.clone()),
        ]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        response.as_string()
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for LSetCommand {
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