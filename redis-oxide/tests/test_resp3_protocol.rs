//! Integration tests for RESP3 protocol support

use redis_oxide::{ConnectionConfig, ProtocolVersion, Resp3Value};

fn assert_resp3_round_trip(expected: &Resp3Value) -> Result<(), redis_oxide::RedisError> {
    use redis_oxide::protocol::resp3::{Resp3Decoder, Resp3Encoder};

    let mut serializer = Resp3Encoder::new();
    let wire_data = serializer.encode(expected)?;
    let mut parser = Resp3Decoder::new();
    let actual = parser.decode(&wire_data)?;
    assert_eq!(expected, &actual);

    Ok(())
}

#[test]
fn test_resp3_basic_data_types() -> Result<(), Box<dyn std::error::Error>> {
    // Test Boolean
    let bool_val = Resp3Value::Boolean(true);
    assert_resp3_round_trip(&bool_val)?;

    // Test Double
    let double_val = Resp3Value::Double(std::f64::consts::PI);
    assert_resp3_round_trip(&double_val)?;

    // Test Map
    let map_val = Resp3Value::Map(vec![
        (
            Resp3Value::SimpleString("key1".into()),
            Resp3Value::SimpleString("value1".into()),
        ),
        (Resp3Value::Number(2), Resp3Value::Number(42)),
    ]);
    assert_resp3_round_trip(&map_val)?;

    // Test Set
    let set_val = Resp3Value::Set(vec![
        Resp3Value::SimpleString("item1".into()),
        Resp3Value::SimpleString("item2".into()),
        Resp3Value::Number(123),
    ]);
    assert_resp3_round_trip(&set_val)?;

    Ok(())
}

#[test]
fn test_resp3_verbatim_string() -> Result<(), Box<dyn std::error::Error>> {
    // Test VerbatimString
    let verbatim_val = Resp3Value::VerbatimString {
        encoding: "txt".to_string(),
        data: "Hello, World!".to_string(),
    };
    assert_resp3_round_trip(&verbatim_val)?;

    // Test different encoding
    let markdown_val = Resp3Value::VerbatimString {
        encoding: "mkd".to_string(),
        data: "# Markdown Title\n\nSome **bold** text.".to_string(),
    };
    assert_resp3_round_trip(&markdown_val)?;

    Ok(())
}

#[test]
fn test_resp3_big_number() -> Result<(), Box<dyn std::error::Error>> {
    // Test BigNumber
    let big_num = Resp3Value::BigNumber("123456789012345678901234567890".to_string());
    assert_resp3_round_trip(&big_num)?;

    Ok(())
}

#[test]
fn test_resp3_attribute() -> Result<(), Box<dyn std::error::Error>> {
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

    assert_resp3_round_trip(&attr_val)?;

    Ok(())
}

#[test]
fn test_resp3_push_type() -> Result<(), Box<dyn std::error::Error>> {
    // Test Push (server-initiated message)
    let push_val = Resp3Value::Push(vec![
        Resp3Value::SimpleString("pubsub".to_string()),
        Resp3Value::SimpleString("message".to_string()),
        Resp3Value::SimpleString("channel1".to_string()),
        Resp3Value::BlobString("Hello from channel!".into()),
    ]);

    assert_resp3_round_trip(&push_val)?;

    Ok(())
}

#[test]
fn test_resp3_null_handling() -> Result<(), Box<dyn std::error::Error>> {
    // Test explicit Null
    let null_value = Resp3Value::Null;
    assert_resp3_round_trip(&null_value)?;
    assert!(null_value.is_null());

    Ok(())
}

#[test]
fn test_resp3_value_conversions() -> Result<(), Box<dyn std::error::Error>> {
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

    let double_val = Resp3Value::Double(std::f64::consts::PI);
    assert_eq!(double_val.as_int()?, 3); // Truncated

    // Test float conversion
    assert!((double_val.as_float()? - std::f64::consts::PI).abs() < f64::EPSILON);
    assert!((num_val.as_float()? - 42.0).abs() < f64::EPSILON);

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

#[test]
fn test_resp3_resp2_compatibility() {
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
}

#[test]
fn test_resp3_type_names() {
    assert_eq!(
        Resp3Value::SimpleString("test".to_string()).type_name(),
        "simple-string"
    );
    assert_eq!(Resp3Value::Number(42).type_name(), "number");
    assert_eq!(Resp3Value::Boolean(true).type_name(), "boolean");
    assert_eq!(
        Resp3Value::Double(std::f64::consts::PI).type_name(),
        "double"
    );
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
}

#[test]
fn test_resp3_complex_nested_structures() -> Result<(), Box<dyn std::error::Error>> {
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

    assert_resp3_round_trip(&complex_val)?;

    Ok(())
}

#[test]
fn test_resp3_error_types() -> Result<(), Box<dyn std::error::Error>> {
    // Test SimpleError
    let simple_error = Resp3Value::SimpleError("ERR something went wrong".to_string());
    assert_resp3_round_trip(&simple_error)?;

    // Test BlobError
    let blob_error = Resp3Value::BlobError("SYNTAX invalid command syntax".into());
    assert_resp3_round_trip(&blob_error)?;

    Ok(())
}

// Note: The following tests would require a Redis 6.0+ server with RESP3 support
// For now, they test the protocol implementation without actual Redis integration

#[test]
fn test_protocol_version_configuration() {
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
}

#[test]
fn test_resp3_encoding_edge_cases() -> Result<(), Box<dyn std::error::Error>> {
    // Test empty string
    let empty_str = Resp3Value::BlobString(bytes::Bytes::new());
    assert_resp3_round_trip(&empty_str)?;

    // Test empty array
    let empty_array = Resp3Value::Array(vec![]);
    assert_resp3_round_trip(&empty_array)?;

    // Test empty map
    let empty_map = Resp3Value::Map(vec![]);
    assert_resp3_round_trip(&empty_map)?;

    // Test empty set
    let empty_set = Resp3Value::Set(vec![]);
    assert_resp3_round_trip(&empty_set)?;

    // Test zero and negative numbers
    let zero = Resp3Value::Number(0);
    assert_resp3_round_trip(&zero)?;

    let negative = Resp3Value::Number(-42);
    assert_resp3_round_trip(&negative)?;

    // Test special float values
    let zero_float = Resp3Value::Double(0.0);
    assert_resp3_round_trip(&zero_float)?;

    let negative_float = Resp3Value::Double(-std::f64::consts::PI);
    assert_resp3_round_trip(&negative_float)?;

    Ok(())
}

#[test]
fn test_resp3_equality() {
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
}
