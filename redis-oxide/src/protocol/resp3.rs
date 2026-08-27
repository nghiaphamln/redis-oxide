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
//! - **`BigNumber`**: Arbitrary precision numbers
//! - **`VerbatimString`**: Strings with encoding information
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
            Self::SimpleString(s) | Self::BigNumber(s) => Ok(s.clone()),
            Self::BlobString(bytes) => String::from_utf8(bytes.to_vec())
                .map_err(|error| RedisError::Type(format!("Invalid UTF-8: {error}"))),
            Self::VerbatimString { data, .. } => Ok(data.clone()),
            Self::Number(n) => Ok(n.to_string()),
            Self::Double(f) => Ok(f.to_string()),
            Self::Boolean(b) => Ok(b.to_string()),
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
            Self::Number(n) => Ok(*n),
            Self::SimpleString(s) => s
                .parse::<i64>()
                .map_err(|e| RedisError::Type(format!("Cannot parse '{s}' to i64: {e}"))),
            Self::BlobString(bytes) => String::from_utf8(bytes.to_vec())
                .map_err(|error| RedisError::Type(format!("Invalid UTF-8: {error}")))?
                .parse::<i64>()
                .map_err(|error| RedisError::Type(format!("Cannot parse blob to i64: {error}"))),
            Self::Double(f) => {
                f.trunc().to_string().parse::<i64>().map_err(|error| {
                    RedisError::Type(format!("Cannot convert {f} to i64: {error}"))
                })
            }
            Self::Boolean(true) => Ok(1),
            Self::Boolean(false) => Ok(0),
            _ => Err(RedisError::Type(format!(
                "Cannot convert {self:?} to integer"
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
            Self::Number(n) => n
                .to_string()
                .parse::<f64>()
                .map_err(|error| RedisError::Type(format!("Cannot convert {n} to f64: {error}"))),
            Self::SimpleString(s) => s
                .parse::<f64>()
                .map_err(|e| RedisError::Type(format!("Cannot parse '{s}' to f64: {e}"))),
            Self::BlobString(bytes) => String::from_utf8(bytes.to_vec())
                .map_err(|error| RedisError::Type(format!("Invalid UTF-8: {error}")))?
                .parse::<f64>()
                .map_err(|error| RedisError::Type(format!("Cannot parse blob to f64: {error}"))),
            _ => Err(RedisError::Type(format!(
                "Cannot convert {self:?} to float"
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
            _ => Err(RedisError::Type(format!("Cannot convert {self:?} to bool"))),
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
            Resp3Value::Array(values) | Resp3Value::Set(values) | Resp3Value::Push(values) => {
                Self::Array(values.into_iter().map(Into::into).collect())
            }
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
            Resp3Value::Attribute { data, .. } => (*data).into(),
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

    fn write_line(&mut self, type_byte: u8, data: &[u8]) {
        self.buffer.put_u8(type_byte);
        self.buffer.extend_from_slice(data);
        self.buffer.extend_from_slice(b"\r\n");
    }

    fn write_i64_line(&mut self, type_byte: u8, value: i64) {
        let mut number = itoa::Buffer::new();
        self.write_line(type_byte, number.format(value).as_bytes());
    }

    fn write_usize_line(&mut self, type_byte: u8, value: usize) {
        let mut number = itoa::Buffer::new();
        self.write_line(type_byte, number.format(value).as_bytes());
    }

    fn write_blob(&mut self, type_byte: u8, data: &[u8]) {
        self.write_usize_line(type_byte, data.len());
        self.buffer.extend_from_slice(data);
        self.buffer.extend_from_slice(b"\r\n");
    }

    fn encode_sequence(&mut self, type_byte: u8, values: &[Resp3Value]) -> RedisResult<()> {
        self.write_usize_line(type_byte, values.len());
        for value in values {
            self.encode_value(value)?;
        }
        Ok(())
    }

    fn encode_pairs(
        &mut self,
        type_byte: u8,
        pairs: &[(Resp3Value, Resp3Value)],
    ) -> RedisResult<()> {
        self.write_usize_line(type_byte, pairs.len());
        for (key, value) in pairs {
            self.encode_value(key)?;
            self.encode_value(value)?;
        }
        Ok(())
    }

    fn encode_value(&mut self, value: &Resp3Value) -> RedisResult<()> {
        match value {
            Resp3Value::SimpleString(s) => self.write_line(b'+', s.as_bytes()),
            Resp3Value::SimpleError(s) => self.write_line(b'-', s.as_bytes()),
            Resp3Value::Number(n) => self.write_i64_line(b':', *n),
            Resp3Value::BlobString(bytes) => self.write_blob(b'$', bytes),
            Resp3Value::Array(values) => self.encode_sequence(b'*', values)?,
            Resp3Value::Null => {
                self.buffer.extend_from_slice(b"_\r\n");
            }
            Resp3Value::Boolean(flag) => self.write_line(b'#', if *flag { b"t" } else { b"f" }),
            Resp3Value::Double(float) => self.write_line(b',', float.to_string().as_bytes()),
            Resp3Value::BigNumber(s) => self.write_line(b'(', s.as_bytes()),
            Resp3Value::BlobError(bytes) => self.write_blob(b'!', bytes),
            Resp3Value::VerbatimString { encoding, data } => {
                let content = format!("{encoding}:{data}");
                self.write_blob(b'=', content.as_bytes());
            }
            Resp3Value::Map(pairs) => self.encode_pairs(b'%', pairs)?,
            Resp3Value::Set(values) => self.encode_sequence(b'~', values)?,
            Resp3Value::Attribute { attrs, data } => {
                self.encode_pairs(b'|', attrs)?;
                self.encode_value(data)?;
            }
            Resp3Value::Push(values) => self.encode_sequence(b'>', values)?,
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
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be completed.
    pub fn try_decode(&mut self, data: &[u8]) -> RedisResult<Option<Resp3Value>> {
        self.buffer.extend_from_slice(data);
        let mut cursor = Cursor::new(&self.buffer[..]);
        match Self::decode_value(&mut cursor) {
            Ok(value) => {
                let consumed = usize::try_from(cursor.position()).map_err(|_| {
                    RedisError::Protocol("RESP cursor position exceeds platform size".to_string())
                })?;
                self.buffer.advance(consumed);
                Ok(Some(value))
            }
            Err(RedisError::Protocol(message)) if message.starts_with("Incomplete") => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn decode_value(cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        if !cursor.has_remaining() {
            return Err(RedisError::Protocol("Incomplete data".to_string()));
        }

        let type_byte = cursor.get_u8();
        match type_byte {
            b'+' => Self::decode_simple_string(cursor),
            b'-' => Self::decode_simple_error(cursor),
            b':' => Self::decode_number(cursor),
            b'$' => Self::decode_blob_string(cursor),
            b'*' => Self::decode_array(cursor),
            b'_' => Self::decode_null(cursor),
            b'#' => Self::decode_boolean(cursor),
            b',' => Self::decode_double(cursor),
            b'(' => Self::decode_big_number(cursor),
            b'!' => Self::decode_blob_error(cursor),
            b'=' => Self::decode_verbatim_string(cursor),
            b'%' => Self::decode_map(cursor),
            b'~' => Self::decode_set(cursor),
            b'|' => Self::decode_attribute(cursor),
            b'>' => Self::decode_push(cursor),
            _ => Err(RedisError::Protocol(format!(
                "Unknown RESP3 type byte: {}",
                type_byte as char
            ))),
        }
    }

    fn decode_simple_string(cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let line = Self::read_line(cursor)?;
        Ok(Resp3Value::SimpleString(
            Self::line_as_str(line)?.to_owned(),
        ))
    }

    fn decode_simple_error(cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let line = Self::read_line(cursor)?;
        Ok(Resp3Value::SimpleError(Self::line_as_str(line)?.to_owned()))
    }

    fn decode_number(cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let line = Self::read_line(cursor)?;
        let num = Self::parse_i64(line, "number")?;
        Ok(Resp3Value::Number(num))
    }

    fn decode_blob_string(cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let len_line = Self::read_line(cursor)?;
        let len = Self::parse_i64(len_line, "blob string length")?;

        if len == -1 {
            return Ok(Resp3Value::Null);
        }

        if len < 0 {
            return Err(RedisError::Protocol(
                "Invalid blob string length".to_string(),
            ));
        }

        let len = usize::try_from(len)
            .map_err(|_| RedisError::Protocol("Invalid blob string length".to_string()))?;
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

    fn decode_array(cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let len_line = Self::read_line(cursor)?;
        let len = Self::parse_i64(len_line, "array length")?;

        if len == -1 {
            return Ok(Resp3Value::Null);
        }

        if len < 0 {
            return Err(RedisError::Protocol("Invalid array length".to_string()));
        }

        let len = usize::try_from(len)
            .map_err(|_| RedisError::Protocol("Invalid array length".to_string()))?;
        if len > MAX_COLLECTION_ELEMENTS {
            return Err(RedisError::Protocol(format!(
                "Array exceeds maximum element count of {MAX_COLLECTION_ELEMENTS}"
            )));
        }
        let mut array = Vec::with_capacity(len);
        for _ in 0..len {
            array.push(Self::decode_value(cursor)?);
        }

        Ok(Resp3Value::Array(array))
    }

    fn decode_null(cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let line = Self::read_line(cursor)?;
        if line.is_empty() {
            return Ok(Resp3Value::Null);
        }

        Self::line_as_str(line)?;
        Err(RedisError::Protocol("Invalid null format".to_string()))
    }

    fn decode_boolean(cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let line = Self::read_line(cursor)?;
        match line {
            b"t" => Ok(Resp3Value::Boolean(true)),
            b"f" => Ok(Resp3Value::Boolean(false)),
            _ => Err(RedisError::Protocol(format!(
                "Invalid boolean: {}",
                Self::line_as_str(line)?
            ))),
        }
    }

    fn decode_double(cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let line = Self::read_line(cursor)?;
        let num = Self::line_as_str(line)?
            .parse::<f64>()
            .map_err(|e| RedisError::Protocol(format!("Invalid double: {e}")))?;
        Ok(Resp3Value::Double(num))
    }

    fn decode_big_number(cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let line = Self::read_line(cursor)?;
        Ok(Resp3Value::BigNumber(Self::line_as_str(line)?.to_owned()))
    }

    fn decode_blob_error(cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let len_line = Self::read_line(cursor)?;
        let len = Self::parse_usize(len_line, "blob error length")?;

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

    fn decode_verbatim_string(cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let len_line = Self::read_line(cursor)?;
        let len = Self::parse_usize(len_line, "verbatim string length")?;

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

        if &cursor.chunk()[len..len + 2] != b"\r\n" {
            return Err(RedisError::Protocol(
                "Invalid verbatim string terminator".to_string(),
            ));
        }

        let (encoding, data) = {
            let content = std::str::from_utf8(&cursor.chunk()[..len]).map_err(|error| {
                RedisError::Protocol(format!("Invalid UTF-8 in verbatim string: {error}"))
            })?;
            let Some((encoding, data)) = content.split_once(':') else {
                return Err(RedisError::Protocol(
                    "Invalid verbatim string format".to_string(),
                ));
            };
            (encoding.to_owned(), data.to_owned())
        };

        cursor.advance(len + 2);
        Ok(Resp3Value::VerbatimString { encoding, data })
    }

    fn decode_map(cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let len_line = Self::read_line(cursor)?;
        let len = Self::parse_usize(len_line, "map length")?;

        if len > MAX_COLLECTION_ELEMENTS {
            return Err(RedisError::Protocol(format!(
                "Map exceeds maximum element count of {MAX_COLLECTION_ELEMENTS}"
            )));
        }

        let mut map = Vec::with_capacity(len);
        for _ in 0..len {
            let key = Self::decode_value(cursor)?;
            let value = Self::decode_value(cursor)?;
            map.push((key, value));
        }

        Ok(Resp3Value::Map(map))
    }

    fn decode_set(cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let len_line = Self::read_line(cursor)?;
        let len = Self::parse_usize(len_line, "set length")?;

        if len > MAX_COLLECTION_ELEMENTS {
            return Err(RedisError::Protocol(format!(
                "Set exceeds maximum element count of {MAX_COLLECTION_ELEMENTS}"
            )));
        }

        let mut set = Vec::with_capacity(len);
        for _ in 0..len {
            let value = Self::decode_value(cursor)?;
            set.push(value);
        }

        Ok(Resp3Value::Set(set))
    }

    fn decode_attribute(cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let len_line = Self::read_line(cursor)?;
        let len = Self::parse_usize(len_line, "attribute length")?;

        if len > MAX_COLLECTION_ELEMENTS {
            return Err(RedisError::Protocol(format!(
                "Attribute exceeds maximum element count of {MAX_COLLECTION_ELEMENTS}"
            )));
        }

        let mut attrs = Vec::with_capacity(len);
        for _ in 0..len {
            let key = Self::decode_value(cursor)?;
            let value = Self::decode_value(cursor)?;
            attrs.push((key, value));
        }

        let data = Box::new(Self::decode_value(cursor)?);
        Ok(Resp3Value::Attribute { attrs, data })
    }

    fn decode_push(cursor: &mut Cursor<&[u8]>) -> RedisResult<Resp3Value> {
        let len_line = Self::read_line(cursor)?;
        let len = Self::parse_usize(len_line, "push length")?;

        if len > MAX_COLLECTION_ELEMENTS {
            return Err(RedisError::Protocol(format!(
                "Push exceeds maximum element count of {MAX_COLLECTION_ELEMENTS}"
            )));
        }

        let mut array = Vec::with_capacity(len);
        for _ in 0..len {
            array.push(Self::decode_value(cursor)?);
        }

        Ok(Resp3Value::Push(array))
    }

    fn line_as_str(line: &[u8]) -> RedisResult<&str> {
        std::str::from_utf8(line)
            .map_err(|error| RedisError::Protocol(format!("Invalid UTF-8 in line: {error}")))
    }

    fn parse_i64(line: &[u8], value_type: &str) -> RedisResult<i64> {
        Self::line_as_str(line)?
            .parse::<i64>()
            .map_err(|error| RedisError::Protocol(format!("Invalid {value_type}: {error}")))
    }

    fn parse_usize(line: &[u8], value_type: &str) -> RedisResult<usize> {
        Self::line_as_str(line)?
            .parse::<usize>()
            .map_err(|error| RedisError::Protocol(format!("Invalid {value_type}: {error}")))
    }

    fn read_line<'a>(cursor: &mut Cursor<&'a [u8]>) -> RedisResult<&'a [u8]> {
        let start = usize::try_from(cursor.position()).map_err(|_| {
            RedisError::Protocol("RESP cursor position exceeds platform size".to_string())
        })?;
        let data = *cursor.get_ref();

        if start >= data.len() {
            return Err(RedisError::Protocol("Incomplete line".to_string()));
        }
        for (offset, window) in data[start..].windows(2).enumerate() {
            if window == b"\r\n" {
                let end = start + offset;
                let line = &data[start..end];
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

    fn assert_round_trip(expected: &Resp3Value) {
        let mut serializer = Resp3Encoder::new();
        let wire_data = serializer.encode(expected).unwrap();
        let mut parser = Resp3Decoder::new();
        let actual = parser.decode(&wire_data).unwrap();

        assert_eq!(expected, &actual);
    }

    #[test]
    fn test_encode_decode_simple_string() {
        assert_round_trip(&Resp3Value::SimpleString("OK".to_string()));
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
    fn rejects_invalid_text_and_header_frames() {
        let mut invalid_simple_string = Resp3Decoder::new();
        assert!(invalid_simple_string.decode(b"+\xff\r\n").is_err());

        let mut invalid_blob_header = Resp3Decoder::new();
        assert!(invalid_blob_header.decode(b"$\xff\r\n").is_err());

        let mut invalid_boolean = Resp3Decoder::new();
        assert!(invalid_boolean.decode(b"#x\r\n").is_err());

        let mut invalid_null = Resp3Decoder::new();
        assert!(invalid_null.decode(b"_x\r\n").is_err());
    }

    #[test]
    fn test_encode_decode_number() {
        assert_round_trip(&Resp3Value::Number(42));
    }

    #[test]
    fn encodes_integer_bounds_and_collection_headers() {
        let mut encoder = Resp3Encoder::new();

        let minimum = encoder.encode(&Resp3Value::Number(i64::MIN)).unwrap();
        assert_eq!(&minimum[..], b":-9223372036854775808\r\n");

        let maximum = encoder.encode(&Resp3Value::Number(i64::MAX)).unwrap();
        assert_eq!(&maximum[..], b":9223372036854775807\r\n");

        let push = encoder
            .encode(&Resp3Value::Push(vec![Resp3Value::Null; 10]))
            .unwrap();
        assert_eq!(&push[..5], b">10\r\n");
    }

    #[test]
    fn test_encode_decode_boolean() {
        assert_round_trip(&Resp3Value::Boolean(true));
    }

    #[test]
    fn test_encode_decode_double() {
        assert_round_trip(&Resp3Value::Double(std::f64::consts::PI));
    }

    #[test]
    fn test_encode_decode_map() {
        assert_round_trip(&Resp3Value::Map(vec![
            (
                Resp3Value::SimpleString("key1".into()),
                Resp3Value::Number(1),
            ),
            (
                Resp3Value::SimpleString("key2".into()),
                Resp3Value::SimpleString("value2".into()),
            ),
        ]));
    }

    #[test]
    fn test_encode_decode_set() {
        assert_round_trip(&Resp3Value::Set(vec![
            Resp3Value::SimpleString("apple".into()),
            Resp3Value::SimpleString("banana".into()),
        ]));
    }

    #[test]
    fn test_encode_decode_array() {
        assert_round_trip(&Resp3Value::Array(vec![
            Resp3Value::SimpleString("hello".to_string()),
            Resp3Value::Number(42),
            Resp3Value::Boolean(true),
        ]));
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
