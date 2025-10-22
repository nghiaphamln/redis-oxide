//! RESP2 protocol implementation
//!
//! This module implements the Redis Serialization Protocol (RESP2) for
//! encoding and decoding Redis commands and responses.

use bytes::{Buf, BufMut, Bytes, BytesMut};
use redis_oxide_core::{
    error::{RedisError, RedisResult},
    value::RespValue,
};
use std::io::Cursor;

const CRLF: &[u8] = b"\r\n";

/// Encodes a RESP value into bytes
pub struct RespEncoder;

impl RespEncoder {
    /// Encode a RESP value into a buffer
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
                buf.put_slice(i.to_string().as_bytes());
                buf.put_slice(CRLF);
            }
            RespValue::BulkString(data) => {
                buf.put_u8(b'$');
                buf.put_slice(data.len().to_string().as_bytes());
                buf.put_slice(CRLF);
                buf.put_slice(data);
                buf.put_slice(CRLF);
            }
            RespValue::Null => {
                buf.put_slice(b"$-1\r\n");
            }
            RespValue::Array(arr) => {
                buf.put_u8(b'*');
                buf.put_slice(arr.len().to_string().as_bytes());
                buf.put_slice(CRLF);
                for item in arr {
                    Self::encode(item, buf)?;
                }
            }
        }
        Ok(())
    }

    /// Encode a command with arguments
    pub fn encode_command(command: &str, args: &[RespValue]) -> RedisResult<Bytes> {
        let mut buf = BytesMut::new();

        // Create array with command + args
        let total_len = 1 + args.len();
        buf.put_u8(b'*');
        buf.put_slice(total_len.to_string().as_bytes());
        buf.put_slice(CRLF);

        // Encode command
        buf.put_u8(b'$');
        buf.put_slice(command.len().to_string().as_bytes());
        buf.put_slice(CRLF);
        buf.put_slice(command.as_bytes());
        buf.put_slice(CRLF);

        // Encode arguments
        for arg in args {
            Self::encode(arg, &mut buf)?;
        }

        Ok(buf.freeze())
    }
}

/// Decodes RESP values from bytes
pub struct RespDecoder;

impl RespDecoder {
    /// Decode a RESP value from a buffer
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
            Ok(Some(RespValue::SimpleString(
                String::from_utf8(line.to_vec())
                    .map_err(|e| RedisError::Protocol(format!("Invalid UTF-8: {}", e)))?,
            )))
        } else {
            Ok(None)
        }
    }

    fn decode_error(buf: &mut Cursor<&[u8]>) -> RedisResult<Option<RespValue>> {
        buf.advance(1); // Skip '-'

        if let Some(line) = Self::read_line(buf)? {
            Ok(Some(RespValue::Error(
                String::from_utf8(line.to_vec())
                    .map_err(|e| RedisError::Protocol(format!("Invalid UTF-8: {}", e)))?,
            )))
        } else {
            Ok(None)
        }
    }

    fn decode_integer(buf: &mut Cursor<&[u8]>) -> RedisResult<Option<RespValue>> {
        buf.advance(1); // Skip ':'

        if let Some(line) = Self::read_line(buf)? {
            let num_str = String::from_utf8(line.to_vec())
                .map_err(|e| RedisError::Protocol(format!("Invalid UTF-8: {}", e)))?;
            let num = num_str
                .parse::<i64>()
                .map_err(|e| RedisError::Protocol(format!("Invalid integer: {}", e)))?;
            Ok(Some(RespValue::Integer(num)))
        } else {
            Ok(None)
        }
    }

    fn decode_bulk_string(buf: &mut Cursor<&[u8]>) -> RedisResult<Option<RespValue>> {
        buf.advance(1); // Skip '$'

        let len_line = match Self::read_line(buf)? {
            Some(line) => line,
            None => return Ok(None),
        };

        let len_str = String::from_utf8(len_line.to_vec())
            .map_err(|e| RedisError::Protocol(format!("Invalid UTF-8: {}", e)))?;
        let len = len_str
            .parse::<i64>()
            .map_err(|e| RedisError::Protocol(format!("Invalid bulk string length: {}", e)))?;

        if len == -1 {
            return Ok(Some(RespValue::Null));
        }

        let len = len as usize;

        // Check if we have enough data
        if buf.remaining() < len + 2 {
            return Ok(None);
        }

        let data = buf.chunk()[..len].to_vec();
        buf.advance(len);

        // Skip CRLF
        if buf.remaining() < 2 {
            return Ok(None);
        }
        buf.advance(2);

        Ok(Some(RespValue::BulkString(Bytes::from(data))))
    }

    fn decode_array(buf: &mut Cursor<&[u8]>) -> RedisResult<Option<RespValue>> {
        buf.advance(1); // Skip '*'

        let len_line = match Self::read_line(buf)? {
            Some(line) => line,
            None => return Ok(None),
        };

        let len_str = String::from_utf8(len_line.to_vec())
            .map_err(|e| RedisError::Protocol(format!("Invalid UTF-8: {}", e)))?;
        let len = len_str
            .parse::<i64>()
            .map_err(|e| RedisError::Protocol(format!("Invalid array length: {}", e)))?;

        if len == -1 {
            return Ok(Some(RespValue::Null));
        }

        let len = len as usize;
        let mut arr = Vec::with_capacity(len);

        for _ in 0..len {
            match Self::decode(buf)? {
                Some(value) => arr.push(value),
                None => return Ok(None),
            }
        }

        Ok(Some(RespValue::Array(arr)))
    }

    fn read_line(buf: &mut Cursor<&[u8]>) -> RedisResult<Option<Vec<u8>>> {
        let start = buf.position() as usize;
        let slice = buf.get_ref();

        // Find CRLF
        for i in start..slice.len().saturating_sub(1) {
            if slice[i] == b'\r' && slice[i + 1] == b'\n' {
                let line = slice[start..i].to_vec();
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
