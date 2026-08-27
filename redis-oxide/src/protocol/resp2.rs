//! RESP2 protocol implementation
//!
//! This module implements the Redis Serialization Protocol (RESP2) for
//! encoding and decoding Redis commands and responses.

use crate::core::{
    error::{RedisError, RedisResult},
    value::RespValue,
};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::io::Cursor;

const CRLF: &[u8] = b"\r\n";
const MAX_FRAME_ELEMENTS: usize = 16_384;
const MAX_BULK_STRING_LENGTH: usize = 512 * 1024 * 1024;

/// Encodes a RESP value into bytes
pub struct RespEncoder;

impl RespEncoder {
    fn write_i64(buf: &mut BytesMut, value: i64) {
        let mut number = itoa::Buffer::new();
        buf.put_slice(number.format(value).as_bytes());
    }

    fn write_usize(buf: &mut BytesMut, value: usize) {
        let mut number = itoa::Buffer::new();
        buf.put_slice(number.format(value).as_bytes());
    }

    /// Encode a RESP value into a buffer
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be completed.
    pub fn encode(value: &RespValue, buf: &mut BytesMut) -> RedisResult<()> {
        match value {
            RespValue::SimpleString(s) => {
                buf.put_u8(b'+');
                buf.put_slice(s.as_bytes());
                buf.put_slice(CRLF);
            }
            RespValue::Error(e) => {
                buf.put_u8(b'-');
                buf.put_slice(e.as_bytes());
                buf.put_slice(CRLF);
            }
            RespValue::Integer(i) => {
                buf.put_u8(b':');
                Self::write_i64(buf, *i);
                buf.put_slice(CRLF);
            }
            RespValue::BulkString(data) => {
                buf.put_u8(b'$');
                Self::write_usize(buf, data.len());
                buf.put_slice(CRLF);
                buf.put_slice(data);
                buf.put_slice(CRLF);
            }
            RespValue::Null => {
                buf.put_slice(b"$-1\r\n");
            }
            RespValue::Array(arr) => {
                buf.put_u8(b'*');
                Self::write_usize(buf, arr.len());
                buf.put_slice(CRLF);
                for item in arr {
                    Self::encode(item, buf)?;
                }
            }
        }
        Ok(())
    }

    fn encode_bulk_argument(data: &[u8], buf: &mut BytesMut) {
        buf.put_u8(b'$');
        Self::write_usize(buf, data.len());
        buf.put_slice(CRLF);
        buf.put_slice(data);
        buf.put_slice(CRLF);
    }

    fn encode_integer_bulk_argument(value: i64, buf: &mut BytesMut) {
        let mut number = itoa::Buffer::new();
        Self::encode_bulk_argument(number.format(value).as_bytes(), buf);
    }

    fn encode_command_arg(arg: &RespValue, buf: &mut BytesMut) -> RedisResult<()> {
        match arg {
            RespValue::SimpleString(s) | RespValue::Error(s) => {
                Self::encode_bulk_argument(s.as_bytes(), buf);
            }
            RespValue::Integer(i) => {
                Self::encode_integer_bulk_argument(*i, buf);
            }
            RespValue::BulkString(data) => {
                Self::encode_bulk_argument(data, buf);
            }
            RespValue::Null => {
                buf.put_slice(b"$-1\r\n");
            }
            RespValue::Array(_) => {
                Self::encode(arg, buf)?;
            }
        }
        Ok(())
    }

    /// Encode a command with arguments
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be completed.
    pub fn encode_command(command: &str, args: &[RespValue]) -> RedisResult<Bytes> {
        let mut buf = BytesMut::new();

        // Create array with command + args
        let total_len = 1 + args.len();
        buf.put_u8(b'*');
        Self::write_usize(&mut buf, total_len);
        buf.put_slice(CRLF);

        // Encode command
        buf.put_u8(b'$');
        Self::write_usize(&mut buf, command.len());
        buf.put_slice(CRLF);
        buf.put_slice(command.as_bytes());
        buf.put_slice(CRLF);

        // Encode arguments
        for arg in args {
            Self::encode_command_arg(arg, &mut buf)?;
        }

        Ok(buf.freeze())
    }
}

/// Decodes RESP values from bytes
pub struct RespDecoder;

impl RespDecoder {
    /// Decode a RESP value from a buffer
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be completed.
    pub fn decode(buf: &mut Cursor<&[u8]>) -> RedisResult<Option<RespValue>> {
        if !buf.has_remaining() {
            return Ok(None);
        }

        let type_byte = buf.chunk()[0];

        match type_byte {
            b'+' => Self::decode_simple_string(buf),
            b'-' => Self::decode_error(buf),
            b':' => Self::decode_integer(buf),
            b'$' => Self::decode_bulk_string(buf),
            b'*' => Self::decode_array(buf),
            _ => Err(RedisError::Protocol(format!(
                "Invalid RESP type byte: {}",
                type_byte as char
            ))),
        }
    }

    fn decode_simple_string(buf: &mut Cursor<&[u8]>) -> RedisResult<Option<RespValue>> {
        buf.advance(1); // Skip '+'

        if let Some(line) = Self::read_line(buf)? {
            Ok(Some(RespValue::SimpleString(Self::line_to_string(line)?)))
        } else {
            Ok(None)
        }
    }

    fn decode_error(buf: &mut Cursor<&[u8]>) -> RedisResult<Option<RespValue>> {
        buf.advance(1); // Skip '-'

        if let Some(line) = Self::read_line(buf)? {
            Ok(Some(RespValue::Error(Self::line_to_string(line)?)))
        } else {
            Ok(None)
        }
    }

    fn decode_integer(buf: &mut Cursor<&[u8]>) -> RedisResult<Option<RespValue>> {
        buf.advance(1); // Skip ':'

        if let Some(line) = Self::read_line(buf)? {
            let num = Self::parse_i64(line, "integer")?;
            Ok(Some(RespValue::Integer(num)))
        } else {
            Ok(None)
        }
    }

    fn decode_bulk_string(buf: &mut Cursor<&[u8]>) -> RedisResult<Option<RespValue>> {
        buf.advance(1); // Skip '$'

        let Some(len_line) = Self::read_line(buf)? else {
            return Ok(None);
        };

        let len = Self::parse_i64(len_line, "bulk string length")?;

        if len == -1 {
            return Ok(Some(RespValue::Null));
        }
        if len < -1 {
            return Err(RedisError::Protocol(
                "Invalid negative bulk string length".to_string(),
            ));
        }

        let len = usize::try_from(len)
            .map_err(|_| RedisError::Protocol("Invalid bulk string length".to_string()))?;
        if len > MAX_BULK_STRING_LENGTH {
            return Err(RedisError::Protocol(format!(
                "Bulk string exceeds maximum size of {MAX_BULK_STRING_LENGTH} bytes"
            )));
        }

        // Check if we have enough data
        let required = len
            .checked_add(2)
            .ok_or_else(|| RedisError::Protocol("Bulk string length overflow".to_string()))?;
        if buf.remaining() < required {
            return Ok(None);
        }

        let data = buf.chunk()[..len].to_vec();
        buf.advance(len);

        if &buf.chunk()[..2] != CRLF {
            return Err(RedisError::Protocol(
                "Missing CRLF after bulk string".to_string(),
            ));
        }
        buf.advance(2);

        Ok(Some(RespValue::BulkString(Bytes::from(data))))
    }

    fn decode_array(buf: &mut Cursor<&[u8]>) -> RedisResult<Option<RespValue>> {
        buf.advance(1); // Skip '*'

        let Some(len_line) = Self::read_line(buf)? else {
            return Ok(None);
        };

        let len = Self::parse_i64(len_line, "array length")?;

        if len == -1 {
            return Ok(Some(RespValue::Null));
        }
        if len < -1 {
            return Err(RedisError::Protocol(
                "Invalid negative array length".to_string(),
            ));
        }

        let len = usize::try_from(len)
            .map_err(|_| RedisError::Protocol("Invalid array length".to_string()))?;
        if len > MAX_FRAME_ELEMENTS {
            return Err(RedisError::Protocol(format!(
                "Array exceeds maximum element count of {MAX_FRAME_ELEMENTS}"
            )));
        }
        let mut arr = Vec::with_capacity(len);

        for _ in 0..len {
            match Self::decode(buf)? {
                Some(value) => arr.push(value),
                None => return Ok(None),
            }
        }

        Ok(Some(RespValue::Array(arr)))
    }

    fn line_as_str(line: &[u8]) -> RedisResult<&str> {
        std::str::from_utf8(line)
            .map_err(|error| RedisError::Protocol(format!("Invalid UTF-8: {error}")))
    }

    fn line_to_string(line: &[u8]) -> RedisResult<String> {
        String::from_utf8(line.to_vec())
            .map_err(|error| RedisError::Protocol(format!("Invalid UTF-8: {error}")))
    }

    fn parse_i64(line: &[u8], value_type: &str) -> RedisResult<i64> {
        Self::line_as_str(line)?
            .parse::<i64>()
            .map_err(|error| RedisError::Protocol(format!("Invalid {value_type}: {error}")))
    }

    fn read_line<'a>(buf: &mut Cursor<&'a [u8]>) -> RedisResult<Option<&'a [u8]>> {
        let start = usize::try_from(buf.position()).map_err(|_| {
            RedisError::Protocol("RESP cursor position exceeds platform size".to_string())
        })?;
        let slice = *buf.get_ref();

        // Find CRLF
        for i in start..slice.len().saturating_sub(1) {
            if slice[i] == b'\r' && slice[i + 1] == b'\n' {
                let line = &slice[start..i];
                buf.set_position((i + 2) as u64);
                return Ok(Some(line));
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_simple_string() {
        let mut buf = BytesMut::new();
        let value = RespValue::SimpleString("OK".to_string());
        RespEncoder::encode(&value, &mut buf).unwrap();
        assert_eq!(&buf[..], b"+OK\r\n");
    }

    #[test]
    fn test_encode_error() {
        let mut buf = BytesMut::new();
        let value = RespValue::Error("ERR unknown command".to_string());
        RespEncoder::encode(&value, &mut buf).unwrap();
        assert_eq!(&buf[..], b"-ERR unknown command\r\n");
    }

    #[test]
    fn test_encode_integer() {
        let mut buf = BytesMut::new();
        let value = RespValue::Integer(1000);
        RespEncoder::encode(&value, &mut buf).unwrap();
        assert_eq!(&buf[..], b":1000\r\n");
    }

    #[test]
    fn encodes_integer_bounds_and_collection_headers() {
        let cases: &[(i64, &[u8])] = &[
            (i64::MIN, b":-9223372036854775808\r\n"),
            (i64::MAX, b":9223372036854775807\r\n"),
        ];

        for (value, expected) in cases {
            let mut buf = BytesMut::new();
            RespEncoder::encode(&RespValue::Integer(*value), &mut buf).unwrap();
            assert_eq!(&buf[..], *expected);
        }

        let mut buf = BytesMut::new();
        RespEncoder::encode(&RespValue::Array(vec![RespValue::Null; 10]), &mut buf).unwrap();
        assert_eq!(&buf[..5], b"*10\r\n");
    }

    #[test]
    fn test_encode_bulk_string() {
        let mut buf = BytesMut::new();
        let value = RespValue::BulkString(Bytes::from("foobar"));
        RespEncoder::encode(&value, &mut buf).unwrap();
        assert_eq!(&buf[..], b"$6\r\nfoobar\r\n");
    }

    #[test]
    fn test_encode_null() {
        let mut buf = BytesMut::new();
        let value = RespValue::Null;
        RespEncoder::encode(&value, &mut buf).unwrap();
        assert_eq!(&buf[..], b"$-1\r\n");
    }

    #[test]
    fn test_encode_array() {
        let mut buf = BytesMut::new();
        let value = RespValue::Array(vec![
            RespValue::BulkString(Bytes::from("foo")),
            RespValue::BulkString(Bytes::from("bar")),
        ]);
        RespEncoder::encode(&value, &mut buf).unwrap();
        assert_eq!(&buf[..], b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n");
    }

    #[test]
    fn test_encode_command() {
        let bytes =
            RespEncoder::encode_command("GET", &[RespValue::BulkString(Bytes::from("mykey"))])
                .unwrap();
        assert_eq!(&bytes[..], b"*2\r\n$3\r\nGET\r\n$5\r\nmykey\r\n");
    }

    #[test]
    fn test_encode_command_arguments_are_bulk_strings() {
        let bytes = RespEncoder::encode_command(
            "LRANGE",
            &[
                RespValue::from("items"),
                RespValue::from(0),
                RespValue::from(-1),
            ],
        )
        .unwrap();

        assert_eq!(
            &bytes[..],
            b"*4\r\n$6\r\nLRANGE\r\n$5\r\nitems\r\n$1\r\n0\r\n$2\r\n-1\r\n"
        );
    }

    #[test]
    fn test_decode_simple_string() {
        let data = b"+OK\r\n";
        let mut cursor = Cursor::new(&data[..]);
        let value = RespDecoder::decode(&mut cursor).unwrap().unwrap();
        assert_eq!(value, RespValue::SimpleString("OK".to_string()));
    }

    #[test]
    fn test_decode_error() {
        let data = b"-ERR unknown\r\n";
        let mut cursor = Cursor::new(&data[..]);
        let value = RespDecoder::decode(&mut cursor).unwrap().unwrap();
        assert_eq!(value, RespValue::Error("ERR unknown".to_string()));
    }

    #[test]
    fn test_decode_integer() {
        let data = b":1000\r\n";
        let mut cursor = Cursor::new(&data[..]);
        let value = RespDecoder::decode(&mut cursor).unwrap().unwrap();
        assert_eq!(value, RespValue::Integer(1000));
    }

    #[test]
    fn test_decode_bulk_string() {
        let data = b"$6\r\nfoobar\r\n";
        let mut cursor = Cursor::new(&data[..]);
        let value = RespDecoder::decode(&mut cursor).unwrap().unwrap();
        assert_eq!(value, RespValue::BulkString(Bytes::from("foobar")));
    }

    #[test]
    fn test_decode_null() {
        let data = b"$-1\r\n";
        let mut cursor = Cursor::new(&data[..]);
        let value = RespDecoder::decode(&mut cursor).unwrap().unwrap();
        assert_eq!(value, RespValue::Null);
    }

    #[test]
    fn test_decode_array() {
        let data = b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n";
        let mut cursor = Cursor::new(&data[..]);
        let value = RespDecoder::decode(&mut cursor).unwrap().unwrap();
        assert_eq!(
            value,
            RespValue::Array(vec![
                RespValue::BulkString(Bytes::from("foo")),
                RespValue::BulkString(Bytes::from("bar")),
            ])
        );
    }

    #[test]
    fn test_decode_incomplete_data() {
        let data = b"+OK\r";
        let mut cursor = Cursor::new(&data[..]);
        let result = RespDecoder::decode(&mut cursor).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn leaves_incomplete_nested_frames_undecoded() {
        let data = b"*2\r\n+OK\r\n$3\r\nfo";
        let mut cursor = Cursor::new(&data[..]);
        let result = RespDecoder::decode(&mut cursor).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn rejects_invalid_utf8_in_text_and_headers() {
        let mut invalid_simple_string = Cursor::new(&b"+\xff\r\n"[..]);
        assert!(RespDecoder::decode(&mut invalid_simple_string).is_err());

        let mut invalid_bulk_header = Cursor::new(&b"$\xff\r\n"[..]);
        assert!(RespDecoder::decode(&mut invalid_bulk_header).is_err());
    }

    #[test]
    fn rejects_invalid_bulk_lengths_and_terminators() {
        let mut negative = Cursor::new(&b"$-2\r\n"[..]);
        assert!(RespDecoder::decode(&mut negative).is_err());

        let mut invalid_terminator = Cursor::new(&b"$3\r\nfooXX"[..]);
        assert!(RespDecoder::decode(&mut invalid_terminator).is_err());
    }

    #[test]
    fn test_roundtrip() {
        let original = RespValue::Array(vec![
            RespValue::SimpleString("OK".to_string()),
            RespValue::Integer(42),
            RespValue::BulkString(Bytes::from("test")),
            RespValue::Null,
        ]);

        let mut buf = BytesMut::new();
        RespEncoder::encode(&original, &mut buf).unwrap();

        let mut cursor = Cursor::new(&buf[..]);
        let decoded = RespDecoder::decode(&mut cursor).unwrap().unwrap();

        assert_eq!(original, decoded);
    }
}
