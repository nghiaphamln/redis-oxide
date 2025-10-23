//! Command builders for Redis Set operations

use crate::commands::Command;
use crate::core::{
    error::{RedisError, RedisResult},
    value::RespValue,
};
use crate::pipeline::PipelineCommand;
use std::collections::HashSet;

/// Represents the `SADD` command.
#[derive(Debug, Clone)]
pub struct SAddCommand {
    key: String,
    members: Vec<String>,
}

impl SAddCommand {
    /// Create a new `SADD` command.
    #[must_use]
    pub fn new(key: impl Into<String>, members: Vec<String>) -> Self {
        Self {
            key: key.into(),
            members,
        }
    }
}

impl Command for SAddCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "SADD"
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

impl PipelineCommand for SAddCommand {
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

/// Represents the `SREM` command.
#[derive(Debug, Clone)]
pub struct SRemCommand {
    key: String,
    members: Vec<String>,
}

impl SRemCommand {
    /// Create a new `SREM` command.
    #[must_use]
    pub fn new(key: impl Into<String>, members: Vec<String>) -> Self {
        Self {
            key: key.into(),
            members,
        }
    }
}

impl Command for SRemCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "SREM"
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

impl PipelineCommand for SRemCommand {
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

/// Represents the `SMEMBERS` command.
#[derive(Debug, Clone)]
pub struct SMembersCommand {
    key: String,
}

impl SMembersCommand {
    /// Create a new `SMEMBERS` command.
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }
}

impl Command for SMembersCommand {
    type Output = HashSet<String>;

    fn command_name(&self) -> &str {
        "SMEMBERS"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![RespValue::from(self.key.clone())]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::Array(items) => {
                let mut result = HashSet::new();
                for item in items {
                    match item {
                        RespValue::BulkString(b) => {
                            let s = String::from_utf8(b.to_vec())
                                .map_err(|e| RedisError::Type(format!("Invalid UTF-8: {e}")))?;
                            result.insert(s);
                        }
                        RespValue::Null => {
                            // Skip null values
                        }
                        _ => {
                            return Err(RedisError::Type(format!(
                                "Unexpected item type in SMEMBERS response: {:?}",
                                item
                            )))
                        }
                    }
                }
                Ok(result)
            }
            _ => Err(RedisError::Type(format!(
                "Unexpected response type for SMEMBERS: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for SMembersCommand {
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

/// Represents the `SISMEMBER` command.
#[derive(Debug, Clone)]
pub struct SIsMemberCommand {
    key: String,
    member: String,
}

impl SIsMemberCommand {
    /// Create a new `SISMEMBER` command.
    #[must_use]
    pub fn new(key: impl Into<String>, member: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            member: member.into(),
        }
    }
}

impl Command for SIsMemberCommand {
    type Output = bool;

    fn command_name(&self) -> &str {
        "SISMEMBER"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![
            RespValue::from(self.key.clone()),
            RespValue::from(self.member.clone()),
        ]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::Integer(1) => Ok(true),
            RespValue::Integer(0) => Ok(false),
            _ => Err(RedisError::Type(format!(
                "Unexpected response type for SISMEMBER: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for SIsMemberCommand {
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

/// Represents the `SCARD` command.
#[derive(Debug, Clone)]
pub struct SCardCommand {
    key: String,
}

impl SCardCommand {
    /// Create a new `SCARD` command.
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }
}

impl Command for SCardCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "SCARD"
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

impl PipelineCommand for SCardCommand {
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

/// Represents the `SPOP` command.
#[derive(Debug, Clone)]
pub struct SPopCommand {
    key: String,
}

impl SPopCommand {
    /// Create a new `SPOP` command.
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }
}

impl Command for SPopCommand {
    type Output = Option<String>;

    fn command_name(&self) -> &str {
        "SPOP"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![RespValue::from(self.key.clone())]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::BulkString(b) => String::from_utf8(b.to_vec())
                .map(Some)
                .map_err(|e| RedisError::Type(format!("Invalid UTF-8: {e}"))),
            RespValue::Null => Ok(None),
            _ => Err(RedisError::Type(format!(
                "Unexpected response type for SPOP: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for SPopCommand {
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

/// Represents the `SRANDMEMBER` command.
#[derive(Debug, Clone)]
pub struct SRandMemberCommand {
    key: String,
}

impl SRandMemberCommand {
    /// Create a new `SRANDMEMBER` command.
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }
}

impl Command for SRandMemberCommand {
    type Output = Option<String>;

    fn command_name(&self) -> &str {
        "SRANDMEMBER"
    }

    fn args(&self) -> Vec<RespValue> {
        vec![RespValue::from(self.key.clone())]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::BulkString(b) => String::from_utf8(b.to_vec())
                .map(Some)
                .map_err(|e| RedisError::Type(format!("Invalid UTF-8: {e}"))),
            RespValue::Null => Ok(None),
            _ => Err(RedisError::Type(format!(
                "Unexpected response type for SRANDMEMBER: {:?}",
                response
            ))),
        }
    }

    fn keys(&self) -> Vec<&[u8]> {
        vec![self.key.as_bytes()]
    }
}

impl PipelineCommand for SRandMemberCommand {
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
