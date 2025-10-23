//! Sorted Set commands for Redis
//!
//! This module provides command builders for Redis Sorted Set operations.

use crate::core::{error::RedisResult, value::RespValue};
use crate::pipeline::PipelineCommand;
use super::Command;

/// ZADD command - Add one or more members to a sorted set, or update its score if it already exists
#[derive(Debug, Clone)]
pub struct ZAddCommand {
    key: String,
    members: Vec<(f64, String)>, // (score, member) pairs
}

impl ZAddCommand {
    /// Create a new ZADD command
    pub fn new(key: impl Into<String>, members: Vec<(f64, String)>) -> Self {
        Self {
            key: key.into(),
            members,
        }
    }

    /// Create a new ZADD command with a single member
    pub fn single(key: impl Into<String>, score: f64, member: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            members: vec![(score, member.into())],
        }
    }
}

impl Command for ZAddCommand {
    type Output = i64;

    fn command_name(&self) -> &str {
        "ZADD"
    }

    fn args(&self) -> Vec<RespValue> {
        let mut args = vec![RespValue::from(self.key.as_str())];
        for (score, member) in &self.members {
            args.push(RespValue::from(score.to_string()));
            args.push(RespValue::from(member.as_str()));
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

/// ZREM command - Remove one or more members from a sorted set
#[derive(Debug, Clone)]
pub struct ZRemCommand {
    key: String,
    members: Vec<String>,
}

impl ZRemCommand {
    /// Create a new ZREM command
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
        let mut args = vec![RespValue::from(self.key.as_str())];
        for member in &self.members {
            args.push(RespValue::from(member.as_str()));
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

/// ZRANGE command - Return a range of members in a sorted set, by index
#[derive(Debug, Clone)]
pub struct ZRangeCommand {
    key: String,
    start: i64,
    stop: i64,
    with_scores: bool,
}

impl ZRangeCommand {
    /// Create a new ZRANGE command
    pub fn new(key: impl Into<String>, start: i64, stop: i64) -> Self {
        Self {
            key: key.into(),
            start,
            stop,
            with_scores: false,
        }
    }

    /// Create a new ZRANGE command with scores
    pub fn with_scores(key: impl Into<String>, start: i64, stop: i64) -> Self {
        Self {
            key: key.into(),
            start,
            stop,
            with_scores: true,
        }
    }
}

impl Command for ZRangeCommand {
    type Output = Vec<String>;

    fn command_name(&self) -> &str {
        "ZRANGE"
    }

    fn args(&self) -> Vec<RespValue> {
        let mut args = vec![
            RespValue::from(self.key.as_str()),
            RespValue::from(self.start.to_string()),
            RespValue::from(self.stop.to_string()),
        ];
        if self.with_scores {
            args.push(RespValue::from("WITHSCORES"));
        }
        args
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::Array(items) => {
                let mut result = Vec::new();
                for item in items {
                    match item {
                        RespValue::BulkString(bytes) => {
                            let s = String::from_utf8_lossy(&bytes).to_string();
                            result.push(s);
                        }
                        _ => return Err(crate::core::error::RedisError::Type(format!(
                            "Unexpected item type in ZRANGE response: {:?}",
                            item
                        ))),
                    }
                }
                Ok(result)
            }
            _ => Err(crate::core::error::RedisError::Type(format!(
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

/// ZSCORE command - Get the score associated with the given member in a sorted set
#[derive(Debug, Clone)]
pub struct ZScoreCommand {
    key: String,
    member: String,
}

impl ZScoreCommand {
    /// Create a new ZSCORE command
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
            RespValue::from(self.key.as_str()),
            RespValue::from(self.member.as_str()),
        ]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::Null => Ok(None),
            RespValue::BulkString(bytes) => {
                let s = String::from_utf8_lossy(&bytes);
                match s.parse::<f64>() {
                    Ok(score) => Ok(Some(score)),
                    Err(_) => Err(crate::core::error::RedisError::Type(format!(
                        "Invalid score format: {}",
                        s
                    ))),
                }
            }
            _ => Err(crate::core::error::RedisError::Type(format!(
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

/// ZCARD command - Get the number of members in a sorted set
#[derive(Debug, Clone)]
pub struct ZCardCommand {
    key: String,
}

impl ZCardCommand {
    /// Create a new ZCARD command
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
        vec![RespValue::from(self.key.as_str())]
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

/// ZRANK command - Determine the index of a member in a sorted set
#[derive(Debug, Clone)]
pub struct ZRankCommand {
    key: String,
    member: String,
}

impl ZRankCommand {
    /// Create a new ZRANK command
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
            RespValue::from(self.key.as_str()),
            RespValue::from(self.member.as_str()),
        ]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::Null => Ok(None),
            RespValue::Integer(rank) => Ok(Some(rank)),
            _ => Err(crate::core::error::RedisError::Type(format!(
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

/// ZREVRANK command - Determine the index of a member in a sorted set, with scores ordered from high to low
#[derive(Debug, Clone)]
pub struct ZRevRankCommand {
    key: String,
    member: String,
}

impl ZRevRankCommand {
    /// Create a new ZREVRANK command
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
            RespValue::from(self.key.as_str()),
            RespValue::from(self.member.as_str()),
        ]
    }

    fn parse_response(&self, response: RespValue) -> RedisResult<Self::Output> {
        match response {
            RespValue::Null => Ok(None),
            RespValue::Integer(rank) => Ok(Some(rank)),
            _ => Err(crate::core::error::RedisError::Type(format!(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zadd_command() {
        let cmd = ZAddCommand::new("myzset", vec![(1.0, "member1".to_string()), (2.0, "member2".to_string())]);
        assert_eq!(cmd.command_name(), "ZADD");
        assert_eq!(cmd.keys(), vec![b"myzset"]);
        
        let args = <ZAddCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 5); // key + 2 * (score + member)
    }

    #[test]
    fn test_zadd_single_command() {
        let cmd = ZAddCommand::single("myzset", 1.5, "member1");
        assert_eq!(cmd.command_name(), "ZADD");
        assert_eq!(cmd.keys(), vec![b"myzset"]);
        
        let args = <ZAddCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 3); // key + score + member
    }

    #[test]
    fn test_zrem_command() {
        let cmd = ZRemCommand::new("myzset", vec!["member1".to_string(), "member2".to_string()]);
        assert_eq!(cmd.command_name(), "ZREM");
        assert_eq!(cmd.keys(), vec![b"myzset"]);
        
        let args = <ZRemCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 3); // key + 2 members
    }

    #[test]
    fn test_zrange_command() {
        let cmd = ZRangeCommand::new("myzset", 0, -1);
        assert_eq!(cmd.command_name(), "ZRANGE");
        assert_eq!(cmd.keys(), vec![b"myzset"]);
        
        let args = <ZRangeCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 3); // key + start + stop
    }

    #[test]
    fn test_zrange_command_with_scores() {
        let cmd = ZRangeCommand::with_scores("myzset", 0, -1);
        assert_eq!(cmd.command_name(), "ZRANGE");
        assert_eq!(cmd.keys(), vec![b"myzset"]);
        
        let args = <ZRangeCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 4); // key + start + stop + WITHSCORES
    }

    #[test]
    fn test_zscore_command() {
        let cmd = ZScoreCommand::new("myzset", "member1");
        assert_eq!(cmd.command_name(), "ZSCORE");
        assert_eq!(cmd.keys(), vec![b"myzset"]);
        
        let args = <ZScoreCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 2); // key + member
    }

    #[test]
    fn test_zcard_command() {
        let cmd = ZCardCommand::new("myzset");
        assert_eq!(cmd.command_name(), "ZCARD");
        assert_eq!(cmd.keys(), vec![b"myzset"]);
        
        let args = <ZCardCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 1); // key only
    }

    #[test]
    fn test_zrank_command() {
        let cmd = ZRankCommand::new("myzset", "member1");
        assert_eq!(cmd.command_name(), "ZRANK");
        assert_eq!(cmd.keys(), vec![b"myzset"]);
        
        let args = <ZRankCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 2); // key + member
    }

    #[test]
    fn test_zrevrank_command() {
        let cmd = ZRevRankCommand::new("myzset", "member1");
        assert_eq!(cmd.command_name(), "ZREVRANK");
        assert_eq!(cmd.keys(), vec![b"myzset"]);
        
        let args = <ZRevRankCommand as Command>::args(&cmd);
        assert_eq!(args.len(), 2); // key + member
    }
}
