//! RESP (`REdis` Serialization Protocol) value types

use crate::core::error::{RedisError, RedisResult};
use bytes::Bytes;

/// RESP protocol value
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RespValue {
    /// Simple string: +OK\r\n
    SimpleString(String),
    /// Error: -ERR message\r\n
    Error(String),
    /// Integer: :1000\r\n
    Integer(i64),
    /// Bulk string: $6\r\nfoobar\r\n
    BulkString(Bytes),
    /// Null bulk string: $-1\r\n
    Null,
    /// Array: *2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n
    Array(Vec<RespValue>),
}

impl RespValue {
    /// Convert to a string if possible
    ///
    /// # Errors
    ///
    /// Returns an error if the value cannot be converted to a string.
    pub fn as_string(&self) -> RedisResult<String> {
        match self {
            Self::SimpleString(s) => Ok(s.clone()),
            Self::BulkString(b) => String::from_utf8(b.to_vec())
                .map_err(|e| RedisError::Type(format!("Invalid UTF-8: {e}"))),
            Self::Null => Err(RedisError::Type("Value is null".to_string())),
            _ => Err(RedisError::Type(format!(
                "Cannot convert {self:?} to string"
            ))),
        }
    }

    /// Convert to an integer if possible
    ///
    /// # Errors
    ///
    /// Returns an error if the value cannot be converted to an integer.
    pub fn as_int(&self) -> RedisResult<i64> {
        match self {
            Self::Integer(i) => Ok(*i),
            Self::BulkString(b) => {
                let s = String::from_utf8(b.to_vec())
                    .map_err(|e| RedisError::Type(format!("Invalid UTF-8: {e}")))?;
                s.parse::<i64>()
                    .map_err(|e| RedisError::Type(format!("Cannot parse integer: {e}")))
            }
            _ => Err(RedisError::Type(format!(
                "Cannot convert {self:?} to integer"
            ))),
        }
    }

    /// Convert to bytes if possible
    ///
    /// # Errors
    ///
    /// Returns an error if the value cannot be converted to bytes.
    pub fn as_bytes(&self) -> RedisResult<Bytes> {
        match self {
            Self::BulkString(b) => Ok(b.clone()),
            Self::SimpleString(s) => Ok(Bytes::from(s.as_bytes().to_vec())),
            Self::Null => Err(RedisError::Type("Value is null".to_string())),
            _ => Err(RedisError::Type(format!(
                "Cannot convert {self:?} to bytes"
            ))),
        }
    }

    /// Convert to an array if possible
    ///
    /// # Errors
    ///
    /// Returns an error if the value cannot be converted to an array.
    pub fn as_array(&self) -> RedisResult<Vec<Self>> {
        match self {
            Self::Array(arr) => Ok(arr.clone()),
            _ => Err(RedisError::Type(format!(
                "Cannot convert {self:?} to array"
            ))),
        }
    }

    /// Check if this is a null value
    #[must_use]
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Check if this is an error
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    /// Extract error message if this is an error
    #[must_use]
    pub fn into_error(self) -> Option<String> {
        match self {
            Self::Error(msg) => Some(msg),
            _ => None,
        }
    }
}

impl From<String> for RespValue {
    fn from(s: String) -> Self {
        Self::BulkString(Bytes::from(s.into_bytes()))
    }
}
impl From<&str> for RespValue {
    fn from(s: &str) -> Self {
        Self::BulkString(Bytes::from(s.as_bytes().to_vec()))
    }
}
impl From<i64> for RespValue {
    fn from(i: i64) -> Self {
        Self::Integer(i)
    }
}
impl From<Vec<u8>> for RespValue {
    fn from(b: Vec<u8>) -> Self {
        Self::BulkString(Bytes::from(b))
    }
}
impl From<Bytes> for RespValue {
    fn from(b: Bytes) -> Self {
        Self::BulkString(b)
    }
}

impl TryFrom<RespValue> for String {
    type Error = RedisError;

    fn try_from(value: RespValue) -> Result<Self, Self::Error> {
        value.as_string()
    }
}

impl TryFrom<RespValue> for i64 {
    type Error = RedisError;

    fn try_from(value: RespValue) -> Result<Self, Self::Error> {
        value.as_int()
    }
}

impl TryFrom<RespValue> for bool {
    type Error = RedisError;

    fn try_from(value: RespValue) -> Result<Self, Self::Error> {
        match value {
            RespValue::Integer(1) => Ok(true),
            RespValue::Integer(0) => Ok(false),
            RespValue::SimpleString(s) if s == "OK" => Ok(true),
            _ => Err(RedisError::Type(format!(
                "Cannot convert {:?} to bool",
                value
            ))),
        }
    }
}
