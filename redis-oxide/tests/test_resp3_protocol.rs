//! Integration tests for RESP3 protocol support

#![allow(clippy::uninlined_format_args)]
#![allow(clippy::similar_names)]
#![allow(clippy::approx_constant)]
#![allow(clippy::float_cmp)]

use redis_oxide::{Client, ConnectionConfig, ProtocolVersion, Resp3Value};

fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string())
}

#[allow(dead_code)]
async fn setup_client_resp3() -> Result<Client, redis_oxide::RedisError> {
    let config =
        ConnectionConfig::new(redis_url().as_str()).with_protocol_version(ProtocolVersion::Resp3);
    Client::connect(config).await
}

#[allow(dead_code)]
async fn setup_client_resp2() -> Result<Client, redis_oxide::RedisError> {
    let config =
        ConnectionConfig::new(redis_url().as_str()).with_protocol_version(ProtocolVersion::Resp2);
    Client::connect(config).await
}

#[tokio::test]
async fn test_resp3_basic_data_types() -> Result<(), Box<dyn std::error::Error>> {
    use redis_oxide::protocol::resp3::{Resp3Decoder, Resp3Encoder};

    let mut encoder = Resp3Encoder::new();
    let mut decoder = Resp3Decoder::new();

    // Test Boolean
    let bool_val = Resp3Value::Boolean(true);
    let encoded = encoder.encode(&bool_val)?;
    let decoded = decoder.decode(&encoded)?;
    assert_eq!(bool_val, decoded);

    // Test Double
    let double_val = Resp3Value::Double(3.14159);
    let encoded = encoder.encode(&double_val)?;
    let decoded = decoder.decode(&encoded)?;
    assert_eq!(double_val, decoded);

    // Test Map
    let map_val = Resp3Value::Map(vec![
        (
            Resp3Value::SimpleString("key1".into()),
            Resp3Value::SimpleString("value1".into()),
        ),
        (Resp3Value::Number(2), Resp3Value::Number(42)),
    ]);
    let encoded = encoder.encode(&map_val)?;
    let decoded = decoder.decode(&encoded)?;
    assert_eq!(map_val, decoded);

    // Test Set
    let set_val = Resp3Value::Set(vec![
        Resp3Value::SimpleString("item1".into()),
        Resp3Value::SimpleString("item2".into()),
        Resp3Value::Number(123),
    ]);
    let encoded = encoder.encode(&set_val)?;
    let decoded = decoder.decode(&encoded)?;
    assert_eq!(set_val, decoded);

    Ok(())
}

#[tokio::test]
async fn test_resp3_verbatim_string() -> Result<(), Box<dyn std::error::Error>> {
    use redis_oxide::protocol::resp3::{Resp3Decoder, Resp3Encoder};

    let mut encoder = Resp3Encoder::new();
    let mut decoder = Resp3Decoder::new();

    // Test VerbatimString
    let verbatim_val = Resp3Value::VerbatimString {
        encoding: "txt".to_string(),
        data: "Hello, World!".to_string(),
    };
    let encoded = encoder.encode(&verbatim_val)?;
    let decoded = decoder.decode(&encoded)?;
    assert_eq!(verbatim_val, decoded);

    // Test different encoding
    let markdown_val = Resp3Value::VerbatimString {
        encoding: "mkd".to_string(),
        data: "# Markdown Title\n\nSome **bold** text.".to_string(),
    };
    let encoded = encoder.encode(&markdown_val)?;
    let decoded = decoder.decode(&encoded)?;
    assert_eq!(markdown_val, decoded);

    Ok(())
}

#[tokio::test]
async fn test_resp3_big_number() -> Result<(), Box<dyn std::error::Error>> {
    use redis_oxide::protocol::resp3::{Resp3Decoder, Resp3Encoder};

    let mut encoder = Resp3Encoder::new();
    let mut decoder = Resp3Decoder::new();

    // Test BigNumber
    let big_num = Resp3Value::BigNumber("123456789012345678901234567890".to_string());
    let encoded = encoder.encode(&big_num)?;
    let decoded = decoder.decode(&encoded)?;
    assert_eq!(big_num, decoded);

    Ok(())
}

#[tokio::test]
async fn test_resp3_attribute() -> Result<(), Box<dyn std::error::Error>> {
    use redis_oxide::protocol::resp3::{Resp3Decoder, Resp3Encoder};

    let mut encoder = Resp3Encoder::new();
    let mut decoder = Resp3Decoder::new();

    // Test Attribute
    let attrs = vec![
        (
            Resp3Value::SimpleString("ttl".into()),
            Resp3Value::Number(3600),
        ),
        (
            Resp3Value::SimpleString("type".into()),
            Resp3Value::SimpleString("string".into()),
        ),
    ];

    let attr_val = Resp3Value::Attribute {
        attrs,
        data: Box::new(Resp3Value::BlobString("actual_data".into())),
    };

    let encoded = encoder.encode(&attr_val)?;
    let decoded = decoder.decode(&encoded)?;
    assert_eq!(attr_val, decoded);

    Ok(())
}

#[tokio::test]
async fn test_resp3_push_type() -> Result<(), Box<dyn std::error::Error>> {
    use redis_oxide::protocol::resp3::{Resp3Decoder, Resp3Encoder};

    let mut encoder = Resp3Encoder::new();
    let mut decoder = Resp3Decoder::new();

    // Test Push (server-initiated message)
    let push_val = Resp3Value::Push(vec![
        Resp3Value::SimpleString("pubsub".to_string()),
        Resp3Value::SimpleString("message".to_string()),
        Resp3Value::SimpleString("channel1".to_string()),
        Resp3Value::BlobString("Hello from channel!".into()),
    ]);

    let encoded = encoder.encode(&push_val)?;
    let decoded = decoder.decode(&encoded)?;
    assert_eq!(push_val, decoded);

    Ok(())
}

#[tokio::test]
async fn test_resp3_null_handling() -> Result<(), Box<dyn std::error::Error>> {
    use redis_oxide::protocol::resp3::{Resp3Decoder, Resp3Encoder};

    let mut encoder = Resp3Encoder::new();
    let mut decoder = Resp3Decoder::new();

    // Test explicit Null
    let null_val = Resp3Value::Null;
    let encoded = encoder.encode(&null_val)?;
    let decoded = decoder.decode(&encoded)?;
    assert_eq!(null_val, decoded);
    assert!(decoded.is_null());

    Ok(())
}

#[tokio::test]
async fn test_resp3_value_conversions() -> Result<(), Box<dyn std::error::Error>> {
    // Test string conversion
    let str_val = Resp3Value::BlobString("hello".into());
    assert_eq!(str_val.as_string()?, "hello");

    let simple_str = Resp3Value::SimpleString("world".to_string());
    assert_eq!(simple_str.as_string()?, "world");

    let verbatim_str = Resp3Value::VerbatimString {
        encoding: "txt".to_string(),
        data: "test".to_string(),
    };
    assert_eq!(verbatim_str.as_string()?, "test");

    // Test integer conversion
    let num_val = Resp3Value::Number(42);
    assert_eq!(num_val.as_int()?, 42);

    let double_val = Resp3Value::Double(3.14);
    assert_eq!(double_val.as_int()?, 3); // Truncated

    // Test float conversion
    assert_eq!(double_val.as_float()?, 3.14);
    assert_eq!(num_val.as_float()?, 42.0);

    // Test boolean conversion
    let bool_true = Resp3Value::Boolean(true);
    let bool_false = Resp3Value::Boolean(false);
    assert!(bool_true.as_bool()?);
    assert!(!bool_false.as_bool()?);

    let num_one = Resp3Value::Number(1);
    let num_zero = Resp3Value::Number(0);
    assert!(num_one.as_bool()?);
    assert!(!num_zero.as_bool()?);

    Ok(())
}

#[tokio::test]
async fn test_resp3_resp2_compatibility() -> Result<(), Box<dyn std::error::Error>> {
    use redis_oxide::core::value::RespValue;

    // Test RESP3 to RESP2 conversion
    let resp3_bool = Resp3Value::Boolean(true);
    let resp2_val: RespValue = resp3_bool.into();
    match resp2_val {
        RespValue::Integer(1) => {} // Boolean true becomes integer 1
        _ => panic!("Expected integer 1"),
    }

    let resp3_map = Resp3Value::Map(vec![(
        Resp3Value::SimpleString("key".into()),
        Resp3Value::SimpleString("value".into()),
    )]);
    let resp2_val: RespValue = resp3_map.into();
    match resp2_val {
        RespValue::Array(arr) => {
            assert_eq!(arr.len(), 2); // key-value pair becomes array
        }
        _ => panic!("Expected array"),
    }

    // Test RESP2 to RESP3 conversion
    let resp2_str = RespValue::SimpleString("test".to_string());
    let resp3_val: Resp3Value = resp2_str.into();
    match resp3_val {
        Resp3Value::SimpleString(s) => assert_eq!(s, "test"),
        _ => panic!("Expected simple string"),
    }

    Ok(())
}

#[tokio::test]
async fn test_resp3_type_names() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        Resp3Value::SimpleString("test".to_string()).type_name(),
        "simple-string"
    );
    assert_eq!(Resp3Value::Number(42).type_name(), "number");
    assert_eq!(Resp3Value::Boolean(true).type_name(), "boolean");
    assert_eq!(Resp3Value::Double(3.14).type_name(), "double");
    assert_eq!(Resp3Value::Null.type_name(), "null");

    assert_eq!(Resp3Value::Map(vec![]).type_name(), "map");

    assert_eq!(Resp3Value::Set(vec![]).type_name(), "set");

    assert_eq!(
        Resp3Value::BigNumber("123".to_string()).type_name(),
        "big-number"
    );

    let verbatim = Resp3Value::VerbatimString {
        encoding: "txt".to_string(),
        data: "test".to_string(),
    };
    assert_eq!(verbatim.type_name(), "verbatim-string");

    Ok(())
}

#[tokio::test]
async fn test_resp3_complex_nested_structures() -> Result<(), Box<dyn std::error::Error>> {
    use redis_oxide::protocol::resp3::{Resp3Decoder, Resp3Encoder};

    let mut encoder = Resp3Encoder::new();
    let mut decoder = Resp3Decoder::new();

    // Create a complex nested structure
    let inner_map = vec![
        (
            Resp3Value::SimpleString("nested_key".into()),
            Resp3Value::Boolean(true),
        ),
        (
            Resp3Value::SimpleString("nested_number".into()),
            Resp3Value::Double(std::f64::consts::E),
        ),
    ];

    let inner_set = vec![
        Resp3Value::SimpleString("set_item1".into()),
        Resp3Value::Number(999),
    ];

    let outer_map = vec![
        (
            Resp3Value::SimpleString("inner_map".into()),
            Resp3Value::Map(inner_map),
        ),
        (
            Resp3Value::SimpleString("inner_set".into()),
            Resp3Value::Set(inner_set),
        ),
        (
            Resp3Value::SimpleString("simple_value".into()),
            Resp3Value::BlobString("simple".into()),
        ),
    ];

    let complex_val = Resp3Value::Array(vec![
        Resp3Value::Map(outer_map),
        Resp3Value::VerbatimString {
            encoding: "json".to_string(),
            data: r#"{"json": "data"}"#.to_string(),
        },
        Resp3Value::Push(vec![
            Resp3Value::SimpleString("push_type".to_string()),
            Resp3Value::Number(12345),
        ]),
    ]);

    // Test encoding and decoding
    let encoded = encoder.encode(&complex_val)?;
    let decoded = decoder.decode(&encoded)?;
    assert_eq!(complex_val, decoded);

    Ok(())
}

#[tokio::test]
async fn test_resp3_error_types() -> Result<(), Box<dyn std::error::Error>> {
    use redis_oxide::protocol::resp3::{Resp3Decoder, Resp3Encoder};

    let mut encoder = Resp3Encoder::new();
    let mut decoder = Resp3Decoder::new();

    // Test SimpleError
    let simple_error = Resp3Value::SimpleError("ERR something went wrong".to_string());
    let encoded = encoder.encode(&simple_error)?;
    let decoded = decoder.decode(&encoded)?;
    assert_eq!(simple_error, decoded);

    // Test BlobError
    let blob_error = Resp3Value::BlobError("SYNTAX invalid command syntax".into());
    let encoded = encoder.encode(&blob_error)?;
    let decoded = decoder.decode(&encoded)?;
    assert_eq!(blob_error, decoded);

    Ok(())
}

// Note: The following tests would require a Redis 6.0+ server with RESP3 support
// For now, they test the protocol implementation without actual Redis integration

#[tokio::test]
async fn test_protocol_version_configuration() -> Result<(), Box<dyn std::error::Error>> {
    // Test RESP2 configuration (default)
    let config_resp2 = ConnectionConfig::new("redis://localhost:6379")
        .with_protocol_version(ProtocolVersion::Resp2);
    assert_eq!(config_resp2.protocol_version, ProtocolVersion::Resp2);

    // Test RESP3 configuration
    let config_resp3 = ConnectionConfig::new("redis://localhost:6379")
        .with_protocol_version(ProtocolVersion::Resp3);
    assert_eq!(config_resp3.protocol_version, ProtocolVersion::Resp3);

    // Test default is RESP2
    let config_default = ConnectionConfig::new("redis://localhost:6379");
    assert_eq!(config_default.protocol_version, ProtocolVersion::Resp2);

    Ok(())
}

#[tokio::test]
async fn test_resp3_encoding_edge_cases() -> Result<(), Box<dyn std::error::Error>> {
    use redis_oxide::protocol::resp3::{Resp3Decoder, Resp3Encoder};

    let mut encoder = Resp3Encoder::new();
    let mut decoder = Resp3Decoder::new();

    // Test empty string
    let empty_str = Resp3Value::BlobString(bytes::Bytes::new());
    let encoded = encoder.encode(&empty_str)?;
    let decoded = decoder.decode(&encoded)?;
    assert_eq!(empty_str, decoded);

    // Test empty array
    let empty_array = Resp3Value::Array(vec![]);
    let encoded = encoder.encode(&empty_array)?;
    let decoded = decoder.decode(&encoded)?;
    assert_eq!(empty_array, decoded);

    // Test empty map
    let empty_map = Resp3Value::Map(vec![]);
    let encoded = encoder.encode(&empty_map)?;
    let decoded = decoder.decode(&encoded)?;
    assert_eq!(empty_map, decoded);

    // Test empty set
    let empty_set = Resp3Value::Set(vec![]);
    let encoded = encoder.encode(&empty_set)?;
    let decoded = decoder.decode(&encoded)?;
    assert_eq!(empty_set, decoded);

    // Test zero and negative numbers
    let zero = Resp3Value::Number(0);
    let encoded = encoder.encode(&zero)?;
    let decoded = decoder.decode(&encoded)?;
    assert_eq!(zero, decoded);

    let negative = Resp3Value::Number(-42);
    let encoded = encoder.encode(&negative)?;
    let decoded = decoder.decode(&encoded)?;
    assert_eq!(negative, decoded);

    // Test special float values
    let zero_float = Resp3Value::Double(0.0);
    let encoded = encoder.encode(&zero_float)?;
    let decoded = decoder.decode(&encoded)?;
    assert_eq!(zero_float, decoded);

    let negative_float = Resp3Value::Double(-3.14);
    let encoded = encoder.encode(&negative_float)?;
    let decoded = decoder.decode(&encoded)?;
    assert_eq!(negative_float, decoded);

    Ok(())
}

#[tokio::test]
async fn test_resp3_equality() -> Result<(), Box<dyn std::error::Error>> {
    let val1 = Resp3Value::SimpleString("test".to_string());
    let val2 = Resp3Value::SimpleString("test".to_string());
    assert_eq!(val1, val2);

    // Test different types with same content
    let simple_str = Resp3Value::SimpleString("hello".to_string());
    let blob_str = Resp3Value::BlobString("hello".into());
    assert_ne!(simple_str, blob_str); // Different types should not be equal

    // Test boolean values
    let bool_true1 = Resp3Value::Boolean(true);
    let bool_true2 = Resp3Value::Boolean(true);
    let bool_false = Resp3Value::Boolean(false);
    assert_eq!(bool_true1, bool_true2);
    assert_ne!(bool_true1, bool_false);

    Ok(())
}
