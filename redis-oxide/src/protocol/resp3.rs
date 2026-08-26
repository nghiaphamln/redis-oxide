//! RESP3 (Redis Serialization Protocol version 3) implementation
//!
//! RESP3 is the new protocol introduced in Redis 6.0 that extends RESP2 with
//! additional data types and improved semantics. This module provides full
//! RESP3 support while maintaining backward compatibility with RESP2.
//!
//! # New RESP3 Data Types
//!
//! - **Map**: Key-value pairs (similar to hash tables)
//! - **Set**: Unordered collection of unique elements
//! - **Attribute**: Metadata attached to other types
//! - **Push**: Server-initiated messages (pub/sub, monitoring)
//! - **Boolean**: True/false values
//! - **Double**: IEEE 754 floating point numbers
//! - **BigNumber**: Arbitrary precision numbers
//! - **VerbatimString**: Strings with encoding information
//! - **Null**: Explicit null value
//!
//! # Examples
//!
//! ```no_run
//! use redis_oxide::protocol::resp3::{Resp3Value, Resp3Encoder, Resp3Decoder};
//! // Create a RESP3 map
//! let value = Resp3Value::Map(vec![
//!     (
//!         Resp3Value::SimpleString("name".into()),
//!         Resp3Value::BlobString("Alice".into()),
//!     ),
//!     (Resp3Value::SimpleString("age".into()), Resp3Value::Number(30)),
//! ]);
//!
//! // Encode to bytes
//! let mut encoder = Resp3Encoder::new();
//! let encoded = encoder.encode(&value)?;
//!
//! // Decode back
//! let mut decoder = Resp3Decoder::new();
//! let decoded = decoder.decode(&encoded)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::core::{
    error::{RedisError, RedisResult},
    value::RespValue,
};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::io::Cursor;

const MAX_BLOB_LENGTH: usize = 512 * 1024 * 1024;
const MAX_COLLECTION_ELEMENTS: usize = 16_384;

/// RESP3 protocol data types
#[derive(Debug, Clone, PartialEq)]
pub enum Resp3Value {
    /// Simple string: +OK\r\n
    SimpleString(String),
    /// Simple error: -ERR message\r\n
    SimpleError(String),
    /// Number (integer): :123\r\n
    Number(i64),
    /// Blob string: $5\r\nhello\r\n
    BlobString(Bytes),
    /// Array: *3\r\n$3\r\nfoo\r\n$3\r\nbar\r\n$3\r\nbaz\r\n
    Array(Vec<Self>),
    /// Null: _\r\n
    Null,
    /// Boolean: #t\r\n or #f\r\n
    Boolean(bool),
    /// Double: ,1.23\r\n
    Double(f64),
    /// Big number: (3492890328409238509324850943850943825024385\r\n
    BigNumber(String),
    /// Blob error: !21\r\nSYNTAX invalid syntax\r\n
    BlobError(Bytes),
    /// Verbatim string: =15\r\ntxt:Some string\r\n
    VerbatimString {
        /// The encoding type (e.g., "txt", "mkd")
        encoding: String,
        /// The actual string data
        data: String,
    },
    /// Map: %2\r\n+first\r\n:1\r\n+second\r\n:2\r\n.
    ///
    /// RESP3 permits arbitrary values as keys, so pairs retain their original
    /// type and wire order instead of being coerced into a string-keyed map.
    Map(Vec<(Self, Self)>),
    /// Set: ~3\r\n+orange\r\n+apple\r\n+one\r\n.
    ///
    /// Values are retained in wire order; this also permits values such as
    /// doubles that cannot safely implement `Eq` and `Hash`.
    Set(Vec<Self>),
    /// Attribute: |1\r\n+ttl\r\n:3600\r\n+key\r\n+value\r\n
    Attribute {
        /// The attribute key-value pairs
        attrs: Vec<(Self, Self)>,
        /// The actual data with attributes attached
        data: Box<Self>,
    },
    /// Push: >4\r\n+pubsub\r\n+message\r\n+channel\r\n+hello\r\n
    Push(Vec<Self>),
}

impl Resp3Value {
    /// Convert to a string if possible
    ///
    /// # Errors
    ///
    /// Returns an error if the value cannot be converted to a string.
    pub fn as_string(&self) -> RedisResult<String> {
        match self {
            Self::SimpleString(s) => Ok(s.clone()),
            Self::BlobString(bytes) => String::from_utf8(bytes.to_vec())
                .map_err(|error| RedisError::Type(format!("Invalid UTF-8: {error}"))),
            Self::VerbatimString { data, .. } => Ok(data.clone()),
            Self::Number(n) => Ok(n.to_string()),
            Self::Double(f) => Ok(f.to_string()),
            Self::Boolean(b) => Ok(b.to_string()),
            Self::BigNumber(s) => Ok(s.clone()),
            Self::Null => Err(RedisError::Type("Value is null".to_string())),
            _ => Err(RedisError::Type(format!(
                "Cannot convert {:?} to string",
                self
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
            Self::Number(n) => Ok(*n),
            Self::SimpleString(s) => s
                .parse::<i64>()
                .map_err(|e| RedisError::Type(format!("Cannot parse '{}' to i64: {}", s, e))),
            Self::BlobString(bytes) => String::from_utf8(bytes.to_vec())
                .map_err(|error| RedisError::Type(format!("Invalid UTF-8: {error}")))?
                .parse::<i64>()
                .map_err(|error| RedisError::Type(format!("Cannot parse blob to i64: {error}"))),
            Self::Double(f) => Ok(*f as i64),
            Self::Boolean(true) => Ok(1),
            Self::Boolean(false) => Ok(0),
            _ => Err(RedisError::Type(format!(
                "Cannot convert {:?} to integer",
                self
            ))),
        }
    }

    /// Convert to a float if possible
    ///
    /// # Errors
    ///
    /// Returns an error if the value cannot be converted to a float.
    pub fn as_float(&self) -> RedisResult<f64> {
        match self {
            Self::Double(f) => Ok(*f),
            Self::Number(n) => Ok(*n as f64),
            Self::SimpleString(s) => s
                .parse::<f64>()
                .map_err(|e| RedisError::Type(format!("Cannot parse '{}' to f64: {}", s, e))),
            Self::BlobString(bytes) => String::from_utf8(bytes.to_vec())
                .map_err(|error| RedisError::Type(format!("Invalid UTF-8: {error}")))?
                .parse::<f64>()
                .map_err(|error| RedisError::Type(format!("Cannot parse blob to f64: {error}"))),
            _ => Err(RedisError::Type(format!(
                "Cannot convert {:?} to float",
                self
            ))),
        }
    }

    /// Convert to a boolean if possible
    ///
    /// # Errors
    ///
    /// Returns an error if the value cannot be converted to a boolean.
    pub fn as_bool(&self) -> RedisResult<bool> {
        match self {
            Self::Boolean(b) => Ok(*b),
            Self::Number(1) => Ok(true),
            Self::Number(0) => Ok(false),
            Self::SimpleString(s) if s == "OK" => Ok(true),
            _ => Err(RedisError::Type(format!(
                "Cannot convert {:?} to bool",
                self
            ))),
        }
    }

    /// Check if the value is null
    #[must_use]
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Get the type name of the value
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::SimpleString(_) => "simple-string",
            Self::SimpleError(_) => "simple-error",
            Self::Number(_) => "number",
            Self::BlobString(_) => "blob-string",
            Self::Array(_) => "array",
            Self::Null => "null",
            Self::Boolean(_) => "boolean",
            Self::Double(_) => "double",
            Self::BigNumber(_) => "big-number",
            Self::BlobError(_) => "blob-error",
            Self::VerbatimString { .. } => "verbatim-string",
            Self::Map(_) => "map",
            Self::Set(_) => "set",
            Self::Attribute { .. } => "attribute",
            Self::Push(_) => "push",
        }
    }
}

/// Convert RESP3 value to RESP2 value for backward compatibility
impl From<Resp3Value> for RespValue {
    fn from(value: Resp3Value) -> Self {
        match value {
            Resp3Value::SimpleString(s) => Self::SimpleString(s),
            Resp3Value::SimpleError(s) => Self::Error(s),
            Resp3Value::Number(n) => Self::Integer(n),
            Resp3Value::BlobString(bytes) => Self::BulkString(bytes),
            Resp3Value::Array(arr) => Self::Array(arr.into_iter().map(Into::into).collect()),
            Resp3Value::Null => Self::Null,
            Resp3Value::Boolean(true) => Self::Integer(1),
            Resp3Value::Boolean(false) => Self::Integer(0),
            Resp3Value::Double(f) => Self::BulkString(Bytes::from(f.to_string().into_bytes())),
            Resp3Value::BigNumber(s) => Self::BulkString(Bytes::from(s.into_bytes())),
            Resp3Value::BlobError(bytes) => {
                Self::Error(String::from_utf8_lossy(&bytes).into_owned())
            }
            Resp3Value::VerbatimString { data, .. } => {
                Self::BulkString(Bytes::from(data.into_bytes()))
            }
            Resp3Value::Map(map) => {
                let mut arr = Vec::new();
                for (k, v) in map {
                    arr.push(k.into());
                    arr.push(v.into());
                }
                Self::Array(arr)
            }
            Resp3Value::Set(set) => Self::Array(set.into_iter().map(Into::into).collect()),
            Resp3Value::Attribute { data, .. } => (*data).into(),
            Resp3Value::Push(arr) => Self::Array(arr.into_iter().map(Into::into).collect()),
        }
    }
}

/// Convert RESP2 value to RESP3 value
impl From<RespValue> for Resp3Value {
    fn from(value: RespValue) -> Self {
        match value {
            RespValue::SimpleString(s) => Self::SimpleString(s),
            RespValue::Error(s) => Self::SimpleError(s),
            RespValue::Integer(n) => Self::Number(n),
            RespValue::BulkString(b) => Self::BlobString(b),
            RespValue::Array(arr) => Self::Array(arr.into_iter().map(Into::into).collect()),
            RespValue::Null => Self::Null,
        }
    }
}

/// RESP3 protocol encoder
pub struct Resp3Encoder {
    buffer: BytesMut,
}

impl Resp3Encoder {
    /// Create a new RESP3 encoder
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::new(),
        }
    }

    /// Encode a RESP3 value to bytes
    ///
    /// # Errors
    ///
    /// Returns an error if encoding fails.
    pub fn encode(&mut self, value: &Resp3Value) -> RedisResult<Bytes> {
        self.buffer.clear();
        self.encode_value(value)?;
        Ok(self.buffer.split().freeze())
    }

    fn encode_value(&mut self, value: &Resp3Value) -> RedisResult<()> {
        match value {
            Resp3Value::SimpleString(s) => {
                self.buffer.put_u8(b'+');
                self.buffer.extend_from_slice(s.as_bytes());
                self.buffer.extend_from_slice(b"\r\n");
            }
            Resp3Value::SimpleError(s) => {
                self.buffer.put_u8(b'-');
                self.buffer.extend_from_slice(s.as_bytes());
                self.buffer.extend_from_slice(b"\r\n");
            }
            Resp3Value::Number(n) => {
                self.buffer.put_u8(b':');
                self.buffer.extend_from_slice(n.to_string().as_bytes());
                self.buffer.extend_from_slice(b"\r\n");
            }
            Resp3Value::BlobString(s) => {
                self.buffer.put_u8(b'$');
                self.buffer
                    .extend_from_slice(s.len().to_string().as_bytes());
                self.buffer.extend_from_slice(b"\r\n");
                self.buffer.extend_from_slice(s);
                self.buffer.extend_from_slice(b"\r\n");
            }
            Resp3Value::Array(arr) => {
                self.buffer.put_u8(b'*');
                self.buffer
                    .extend_from_slice(arr.len().to_string().as_bytes());
                self.buffer.extend_from_slice(b"\r\n");
                for item in arr {
                    self.encode_value(item)?;
                }
            }
            Resp3Value::Null => {
                self.buffer.extend_from_slice(b"_\r\n");
            }
            Resp3Value::Boolean(b) => {
                self.buffer.put_u8(b'#');
                self.buffer.put_u8(if *b { b't' } else { b'f' });
                self.buffer.extend_from_slice(b"\r\n");
            }
            Resp3Value::Double(f) => {
                self.buffer.put_u8(b',');
                self.buffer.extend_from_slice(f.to_string().as_bytes());
                self.buffer.extend_from_slice(b"\r\n");
            }
            Resp3Value::BigNumber(s) => {
                self.buffer.put_u8(b'(');
                self.buffer.extend_from_slice(s.as_bytes());
                self.buffer.extend_from_slice(b"\r\n");
            }
            Resp3Value::BlobError(s) => {
                self.buffer.put_u8(b'!');
                self.buffer
                    .extend_from_slice(s.len().to_string().as_bytes());
                self.buffer.extend_from_slice(b"\r\n");
                self.buffer.extend_from_slice(s);
                self.buffer.extend_from_slice(b"\r\n");
            }
            Resp3Value::VerbatimString { encoding, data } => {
                let content = format!("{}:{}", encoding, data);
                self.buffer.put_u8(b'=');
                self.buffer
                    .extend_from_slice(content.len().to_string().as_bytes());
                self.buffer.extend_from_slice(b"\r\n");
                self.buffer.extend_from_slice(content.as_bytes());
                self.buffer.extend_from_slice(b"\r\n");
            }
            Resp3Value::Map(map) => {
                self.buffer.put_u8(b'%');
                self.buffer
                    .extend_from_slice(map.len().to_string().as_bytes());
                self.buffer.extend_from_slice(b"\r\n");
                for (k, v) in map {
                    self.encode_value(k)?;
                    self.encode_value(v)?;
                }
            }
            Resp3Value::Set(set) => {
                self.buffer.put_u8(b'~');
                self.buffer
                    .extend_from_slice(set.len().to_string().as_bytes());
                self.buffer.extend_from_slice(b"\r\n");
                for item in set {
                    self.encode_value(item)?;
                }
            }
            Resp3Value::Attribute { attrs, data } => {
                self.buffer.put_u8(b'|');
                self.buffer
                    .extend_from_slice(attrs.len().to_string().as_bytes());
                self.buffer.extend_from_slice(b"\r\n");
                for (k, v) in attrs {
                    self.encode_value(k)?;
                    self.encode_value(v)?;
                }
                self.encode_value(data)?;
            }
            Resp3Value::Push(arr) => {
                self.buffer.put_u8(b'>');
                self.buffer
                    .extend_from_slice(arr.len().to_string().as_bytes());
                self.buffer.extend_from_slice(b"\r\n");
                for item in arr {
                    self.encode_value(item)?;
                }
            }
        }
        Ok(())
    }
}

impl Default for Resp3Encoder {
    fn default() -> Self {
        Self::new()
    }
}

/// RESP3 protocol decoder
pub struct Resp3Decoder {
    buffer: BytesMut,
}

impl Resp3Decoder {
    /// Create a new RESP3 decoder
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::new(),
        }
    }

    /// Decode bytes into a RESP3 value
    ///
    /// # Errors
    ///
    /// Returns an error if decoding fails or data is incomplete.
    pub fn decode(&mut self, data: &[u8]) -> RedisResult<Resp3Value> {
        self.try_decode(data)?
            .ok_or_else(|| RedisError::Protocol("Incomplete RESP3 response".to_string()))
    }

    /// Feed a response fragment and decode one complete RESP3 value when ready.
    ///
    /// Additional complete values remain buffered for the next call.
    pub fn try_decode(&mut self, data: &[u8]) -> RedisResult<Option<Resp3Value>> {
        self.buffer.extend_from_slice(data);
        let mut cursor = Cursor::new(&self.buffer[..]);
        match self.decode_value(&mut cursor) {
            Ok(value) => {
                let consumed = cursor.position() as usize;
                self.buffer.advance(consumed);
                Ok(Some(value))
            }
            Err(RedisError::Protocol(message)) if message.starts_with("Incomplete") => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn decode_value(&self, cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        if !cursor.has_remaining() {
            return Err(RedisError::Protocol("Incomplete data".to_string()));
        }

        let type_byte = cursor.get_u8();
        match type_byte {
            b'+' => self.decode_simple_string(cursor),
            b'-' => self.decode_simple_error(cursor),
            b':' => self.decode_number(cursor),
            b'$' => self.decode_blob_string(cursor),
            b'*' => self.decode_array(cursor),
            b'_' => self.decode_null(cursor),
            b'#' => self.decode_boolean(cursor),
            b',' => self.decode_double(cursor),
            b'(' => self.decode_big_number(cursor),
            b'!' => self.decode_blob_error(cursor),
            b'=' => self.decode_verbatim_string(cursor),
            b'%' => self.decode_map(cursor),
            b'~' => self.decode_set(cursor),
            b'|' => self.decode_attribute(cursor),
            b'>' => self.decode_push(cursor),
            _ => Err(RedisError::Protocol(format!(
                "Unknown RESP3 type byte: {}",
                type_byte as char
            ))),
        }
    }

    fn decode_simple_string(&self, cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let line = self.read_line(cursor)?;
        Ok(Resp3Value::SimpleString(line))
    }

    fn decode_simple_error(&self, cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let line = self.read_line(cursor)?;
        Ok(Resp3Value::SimpleError(line))
    }

    fn decode_number(&self, cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let line = self.read_line(cursor)?;
        let num = line
            .parse::<i64>()
            .map_err(|e| RedisError::Protocol(format!("Invalid number: {}", e)))?;
        Ok(Resp3Value::Number(num))
    }

    fn decode_blob_string(&self, cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let len_line = self.read_line(cursor)?;
        let len = len_line
            .parse::<i64>()
            .map_err(|e| RedisError::Protocol(format!("Invalid blob string length: {}", e)))?;

        if len == -1 {
            return Ok(Resp3Value::Null);
        }

        if len < 0 {
            return Err(RedisError::Protocol(
                "Invalid blob string length".to_string(),
            ));
        }

        let len = len as usize;
        if len > MAX_BLOB_LENGTH {
            return Err(RedisError::Protocol(format!(
                "Blob string exceeds maximum size of {MAX_BLOB_LENGTH} bytes"
            )));
        }
        if cursor.remaining() < len + 2 {
            return Err(RedisError::Protocol("Incomplete blob string".to_string()));
        }

        let mut data = vec![0u8; len];
        cursor.copy_to_slice(&mut data);

        // Skip \r\n
        if cursor.remaining() < 2 || cursor.get_u8() != b'\r' || cursor.get_u8() != b'\n' {
            return Err(RedisError::Protocol(
                "Invalid blob string terminator".to_string(),
            ));
        }

        Ok(Resp3Value::BlobString(Bytes::from(data)))
    }

    fn decode_array(&self, cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let len_line = self.read_line(cursor)?;
        let len = len_line
            .parse::<i64>()
            .map_err(|e| RedisError::Protocol(format!("Invalid array length: {}", e)))?;

        if len == -1 {
            return Ok(Resp3Value::Null);
        }

        if len < 0 {
            return Err(RedisError::Protocol("Invalid array length".to_string()));
        }

        let len = len as usize;
        if len > MAX_COLLECTION_ELEMENTS {
            return Err(RedisError::Protocol(format!(
                "Array exceeds maximum element count of {MAX_COLLECTION_ELEMENTS}"
            )));
        }
        let mut array = Vec::with_capacity(len);
        for _ in 0..len {
            array.push(self.decode_value(cursor)?);
        }

        Ok(Resp3Value::Array(array))
    }

    fn decode_null(&self, cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let line = self.read_line(cursor)?;
        if line.is_empty() {
            Ok(Resp3Value::Null)
        } else {
            Err(RedisError::Protocol("Invalid null format".to_string()))
        }
    }

    fn decode_boolean(&self, cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let line = self.read_line(cursor)?;
        match line.as_str() {
            "t" => Ok(Resp3Value::Boolean(true)),
            "f" => Ok(Resp3Value::Boolean(false)),
            _ => Err(RedisError::Protocol(format!("Invalid boolean: {}", line))),
        }
    }

    fn decode_double(&self, cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let line = self.read_line(cursor)?;
        let num = line
            .parse::<f64>()
            .map_err(|e| RedisError::Protocol(format!("Invalid double: {}", e)))?;
        Ok(Resp3Value::Double(num))
    }

    fn decode_big_number(&self, cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let line = self.read_line(cursor)?;
        Ok(Resp3Value::BigNumber(line))
    }

    fn decode_blob_error(&self, cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let len_line = self.read_line(cursor)?;
        let len = len_line
            .parse::<usize>()
            .map_err(|e| RedisError::Protocol(format!("Invalid blob error length: {}", e)))?;

        if len > MAX_BLOB_LENGTH {
            return Err(RedisError::Protocol(format!(
                "Blob error exceeds maximum size of {MAX_BLOB_LENGTH} bytes"
            )));
        }

        if cursor.remaining() < len + 2 {
            return Err(RedisError::Protocol("Incomplete blob error".to_string()));
        }

        let mut data = vec![0u8; len];
        cursor.copy_to_slice(&mut data);

        // Skip \r\n
        if cursor.remaining() < 2 || cursor.get_u8() != b'\r' || cursor.get_u8() != b'\n' {
            return Err(RedisError::Protocol(
                "Invalid blob error terminator".to_string(),
            ));
        }

        Ok(Resp3Value::BlobError(Bytes::from(data)))
    }

    fn decode_verbatim_string(&self, cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let len_line = self.read_line(cursor)?;
        let len = len_line
            .parse::<usize>()
            .map_err(|e| RedisError::Protocol(format!("Invalid verbatim string length: {}", e)))?;

        if len > MAX_BLOB_LENGTH {
            return Err(RedisError::Protocol(format!(
                "Verbatim string exceeds maximum size of {MAX_BLOB_LENGTH} bytes"
            )));
        }

        if cursor.remaining() < len + 2 {
            return Err(RedisError::Protocol(
                "Incomplete verbatim string".to_string(),
            ));
        }

        let mut data = vec![0u8; len];
        cursor.copy_to_slice(&mut data);

        // Skip \r\n
        if cursor.remaining() < 2 || cursor.get_u8() != b'\r' || cursor.get_u8() != b'\n' {
            return Err(RedisError::Protocol(
                "Invalid verbatim string terminator".to_string(),
            ));
        }

        let content = String::from_utf8(data).map_err(|e| {
            RedisError::Protocol(format!("Invalid UTF-8 in verbatim string: {}", e))
        })?;

        // Parse encoding:data format
        if let Some(colon_pos) = content.find(':') {
            let encoding = content[..colon_pos].to_string();
            let data = content[colon_pos + 1..].to_string();
            Ok(Resp3Value::VerbatimString { encoding, data })
        } else {
            Err(RedisError::Protocol(
                "Invalid verbatim string format".to_string(),
            ))
        }
    }

    fn decode_map(&self, cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let len_line = self.read_line(cursor)?;
        let len = len_line
            .parse::<usize>()
            .map_err(|e| RedisError::Protocol(format!("Invalid map length: {}", e)))?;

        if len > MAX_COLLECTION_ELEMENTS {
            return Err(RedisError::Protocol(format!(
                "Map exceeds maximum element count of {MAX_COLLECTION_ELEMENTS}"
            )));
        }

        let mut map = Vec::with_capacity(len);
        for _ in 0..len {
            let key = self.decode_value(cursor)?;
            let value = self.decode_value(cursor)?;
            map.push((key, value));
        }

        Ok(Resp3Value::Map(map))
    }

    fn decode_set(&self, cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let len_line = self.read_line(cursor)?;
        let len = len_line
            .parse::<usize>()
            .map_err(|e| RedisError::Protocol(format!("Invalid set length: {}", e)))?;

        if len > MAX_COLLECTION_ELEMENTS {
            return Err(RedisError::Protocol(format!(
                "Set exceeds maximum element count of {MAX_COLLECTION_ELEMENTS}"
            )));
        }

        let mut set = Vec::with_capacity(len);
        for _ in 0..len {
            let value = self.decode_value(cursor)?;
            set.push(value);
        }

        Ok(Resp3Value::Set(set))
    }

    fn decode_attribute(&self, cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let len_line = self.read_line(cursor)?;
        let len = len_line
            .parse::<usize>()
            .map_err(|e| RedisError::Protocol(format!("Invalid attribute length: {}", e)))?;

        if len > MAX_COLLECTION_ELEMENTS {
            return Err(RedisError::Protocol(format!(
                "Attribute exceeds maximum element count of {MAX_COLLECTION_ELEMENTS}"
            )));
        }

        let mut attrs = Vec::with_capacity(len);
        for _ in 0..len {
            let key = self.decode_value(cursor)?;
            let value = self.decode_value(cursor)?;
            attrs.push((key, value));
        }

        let data = Box::new(self.decode_value(cursor)?);
        Ok(Resp3Value::Attribute { attrs, data })
    }

    fn decode_push(&self, cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let len_line = self.read_line(cursor)?;
        let len = len_line
            .parse::<usize>()
            .map_err(|e| RedisError::Protocol(format!("Invalid push length: {}", e)))?;

        if len > MAX_COLLECTION_ELEMENTS {
            return Err(RedisError::Protocol(format!(
                "Push exceeds maximum element count of {MAX_COLLECTION_ELEMENTS}"
            )));
        }

        let mut array = Vec::with_capacity(len);
        for _ in 0..len {
            array.push(self.decode_value(cursor)?);
        }

        Ok(Resp3Value::Push(array))
    }

    fn read_line(&self, cursor: &mut Cursor<&[u8]>) -> RedisResult<String> {
        let start = cursor.position() as usize;
        let data = cursor.get_ref();

        if start >= data.len() {
            return Err(RedisError::Protocol("Incomplete line".to_string()));
        }
        for (offset, window) in data[start..].windows(2).enumerate() {
            if window == b"\r\n" {
                let end = start + offset;
                let line = String::from_utf8(data[start..end].to_vec())
                    .map_err(|e| RedisError::Protocol(format!("Invalid UTF-8 in line: {}", e)))?;
                cursor.set_position((end + 2) as u64);
                return Ok(line);
            }
        }

        Err(RedisError::Protocol("Incomplete line".to_string()))
    }
}

impl Default for Resp3Decoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_simple_string() {
        let mut encoder = Resp3Encoder::new();
        let value = Resp3Value::SimpleString("OK".to_string());
        let encoded = encoder.encode(&value).unwrap();

        let mut decoder = Resp3Decoder::new();
        let decoded = decoder.decode(&encoded).unwrap();

        assert_eq!(value, decoded);
    }

    #[test]
    fn buffers_incomplete_fragments_without_losing_data() {
        let mut decoder = Resp3Decoder::new();
        assert!(decoder.try_decode(b"$5\r\nhel").unwrap().is_none());
        assert_eq!(
            decoder.try_decode(b"lo\r\n").unwrap(),
            Some(Resp3Value::BlobString(Bytes::from("hello")))
        );
    }

    #[test]
    fn test_encode_decode_number() {
        let mut encoder = Resp3Encoder::new();
        let value = Resp3Value::Number(42);
        let encoded = encoder.encode(&value).unwrap();

        let mut decoder = Resp3Decoder::new();
        let decoded = decoder.decode(&encoded).unwrap();

        assert_eq!(value, decoded);
    }

    #[test]
    fn test_encode_decode_boolean() {
        let mut encoder = Resp3Encoder::new();
        let value = Resp3Value::Boolean(true);
        let encoded = encoder.encode(&value).unwrap();

        let mut decoder = Resp3Decoder::new();
        let decoded = decoder.decode(&encoded).unwrap();

        assert_eq!(value, decoded);
    }

    #[test]
    fn test_encode_decode_double() {
        let mut encoder = Resp3Encoder::new();
        let value = Resp3Value::Double(3.14);
        let encoded = encoder.encode(&value).unwrap();

        let mut decoder = Resp3Decoder::new();
        let decoded = decoder.decode(&encoded).unwrap();

        assert_eq!(value, decoded);
    }

    #[test]
    fn test_encode_decode_map() {
        let mut encoder = Resp3Encoder::new();
        let value = Resp3Value::Map(vec![
            (
                Resp3Value::SimpleString("key1".into()),
                Resp3Value::Number(1),
            ),
            (
                Resp3Value::SimpleString("key2".into()),
                Resp3Value::SimpleString("value2".into()),
            ),
        ]);
        let encoded = encoder.encode(&value).unwrap();

        let mut decoder = Resp3Decoder::new();
        let decoded = decoder.decode(&encoded).unwrap();

        assert_eq!(value, decoded);
    }

    #[test]
    fn test_encode_decode_set() {
        let mut encoder = Resp3Encoder::new();
        let value = Resp3Value::Set(vec![
            Resp3Value::SimpleString("apple".into()),
            Resp3Value::SimpleString("banana".into()),
        ]);
        let encoded = encoder.encode(&value).unwrap();

        let mut decoder = Resp3Decoder::new();
        let decoded = decoder.decode(&encoded).unwrap();

        assert_eq!(value, decoded);
    }

    #[test]
    fn test_encode_decode_array() {
        let mut encoder = Resp3Encoder::new();
        let value = Resp3Value::Array(vec![
            Resp3Value::SimpleString("hello".to_string()),
            Resp3Value::Number(42),
            Resp3Value::Boolean(true),
        ]);
        let encoded = encoder.encode(&value).unwrap();

        let mut decoder = Resp3Decoder::new();
        let decoded = decoder.decode(&encoded).unwrap();

        assert_eq!(value, decoded);
    }

    #[test]
    fn test_resp2_compatibility() {
        let resp2_value = RespValue::SimpleString("test".to_string());
        let resp3_value: Resp3Value = resp2_value.clone().into();
        let back_to_resp2: RespValue = resp3_value.into();

        assert_eq!(resp2_value, back_to_resp2);
    }

    #[test]
    fn test_value_conversions() {
        let value = Resp3Value::Number(42);
        assert_eq!(value.as_int().unwrap(), 42);
        assert_eq!(value.as_string().unwrap(), "42");

        let value = Resp3Value::Boolean(true);
        assert!(value.as_bool().unwrap());
        assert_eq!(value.as_int().unwrap(), 1);

        let value = Resp3Value::Double(std::f64::consts::PI);
        assert!(
            (value.as_float().unwrap() - std::f64::consts::PI).abs() < 1e-10,
            "Float value differs from PI"
        );
    }
}
