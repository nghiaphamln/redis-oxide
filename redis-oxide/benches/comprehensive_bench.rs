//! Comprehensive benchmarks for redis-oxide

#![allow(missing_docs)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::explicit_iter_loop)]
#![allow(clippy::approx_constant)] // performance optimization
//!
//! This benchmark suite provides baseline measurements for:
//! - Protocol encoding/decoding performance
//! - Connection pool performance
//! - Command execution performance
//! - Memory allocation patterns

use bytes::{Bytes, BytesMut};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use redis_oxide::{
    commands::{
        Command, GetCommand, HSetCommand, LPushCommand, SAddCommand, SetCommand, ZAddCommand,
    },
    core::{error::RedisError, value::RespValue},
    protocol::{
        resp2::{RespDecoder, RespEncoder},
        resp3::{Resp3Decoder, Resp3Encoder, Resp3Value},
    },
};
use std::collections::HashMap;
use std::io::Cursor;

// Test data generation
fn generate_test_string(size: usize) -> String {
    "x".repeat(size)
}

fn generate_test_array(size: usize) -> Vec<RespValue> {
    (0..size)
        .map(|i| RespValue::BulkString(Bytes::from(format!("item_{}", i))))
        .collect()
}

fn generate_resp3_map(size: usize) -> HashMap<String, Resp3Value> {
    (0..size)
        .map(|i| {
            (
                format!("key_{}", i),
                Resp3Value::SimpleString(format!("value_{}", i)),
            )
        })
        .collect()
}

// RESP2 Protocol Benchmarks
fn bench_resp2_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("resp2_encoding");

    // String encoding benchmarks
    for size in [10, 100, 1000, 10000].iter() {
        let test_string = generate_test_string(*size);
        let value = RespValue::BulkString(Bytes::from(test_string));

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::new("bulk_string", size), size, |b, _| {
            b.iter(|| {
                let mut buf = BytesMut::new();
                RespEncoder::encode(black_box(&value), &mut buf).unwrap();
                black_box(buf);
            });
        });
    }

    // Array encoding benchmarks
    for size in [10, 100, 1000].iter() {
        let test_array = generate_test_array(*size);
        let value = RespValue::Array(test_array);

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("array", size), size, |b, _| {
            b.iter(|| {
                let mut buf = BytesMut::new();
                RespEncoder::encode(black_box(&value), &mut buf).unwrap();
                black_box(buf);
            });
        });
    }

    // Command encoding benchmarks
    let commands = vec![
        ("GET", vec![RespValue::from("key")]),
        (
            "SET",
            vec![RespValue::from("key"), RespValue::from("value")],
        ),
        (
            "HSET",
            vec![
                RespValue::from("hash"),
                RespValue::from("field"),
                RespValue::from("value"),
            ],
        ),
        (
            "LPUSH",
            vec![
                RespValue::from("list"),
                RespValue::from("item1"),
                RespValue::from("item2"),
            ],
        ),
    ];

    for (cmd_name, args) in commands {
        group.bench_function(&format!("command_{}", cmd_name.to_lowercase()), |b| {
            b.iter(|| {
                let encoded =
                    RespEncoder::encode_command(black_box(cmd_name), black_box(&args)).unwrap();
                black_box(encoded);
            });
        });
    }

    group.finish();
}

fn bench_resp2_decoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("resp2_decoding");

    // Pre-encode test data
    let test_cases = vec![
        ("simple_string", b"+OK\r\n".to_vec()),
        ("error", b"-ERR unknown command\r\n".to_vec()),
        ("integer", b":42\r\n".to_vec()),
        ("bulk_string_small", b"$5\r\nhello\r\n".to_vec()),
        ("bulk_string_large", {
            let large_string = generate_test_string(10000);
            let mut data = format!("${}\r\n", large_string.len()).into_bytes();
            data.extend_from_slice(large_string.as_bytes());
            data.extend_from_slice(b"\r\n");
            data
        }),
        ("null", b"$-1\r\n".to_vec()),
        ("array_small", b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n".to_vec()),
    ];

    for (name, data) in test_cases {
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_function(name, |b| {
            b.iter(|| {
                let mut cursor = Cursor::new(black_box(&data[..]));
                let result = RespDecoder::decode(&mut cursor).unwrap();
                black_box(result);
            });
        });
    }

    group.finish();
}

// RESP3 Protocol Benchmarks
fn bench_resp3_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("resp3_encoding");

    let test_cases = vec![
        ("boolean_true", Resp3Value::Boolean(true)),
        ("boolean_false", Resp3Value::Boolean(false)),
        ("double", Resp3Value::Double(3.14159)),
        (
            "big_number",
            Resp3Value::BigNumber("123456789012345678901234567890".to_string()),
        ),
        (
            "verbatim_string",
            Resp3Value::VerbatimString {
                encoding: "txt".to_string(),
                data: "Hello, World!".to_string(),
            },
        ),
    ];

    for (name, value) in test_cases {
        group.bench_function(name, |b| {
            b.iter(|| {
                let mut encoder = Resp3Encoder::new();
                let encoded = encoder.encode(black_box(&value)).unwrap();
                black_box(encoded);
            });
        });
    }

    // Map encoding benchmarks
    for size in [10, 100, 1000].iter() {
        let test_map = generate_resp3_map(*size);
        let value = Resp3Value::Map(test_map);

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("map", size), size, |b, _| {
            b.iter(|| {
                let mut encoder = Resp3Encoder::new();
                let encoded = encoder.encode(black_box(&value)).unwrap();
                black_box(encoded);
            });
        });
    }

    group.finish();
}

fn bench_resp3_decoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("resp3_decoding");

    let test_cases = vec![
        ("boolean_true", b"#t\r\n".to_vec()),
        ("boolean_false", b"#f\r\n".to_vec()),
        ("double", b",3.14159\r\n".to_vec()),
        ("null", b"_\r\n".to_vec()),
        (
            "big_number",
            b"(123456789012345678901234567890\r\n".to_vec(),
        ),
        ("verbatim_string", b"=15\r\ntxt:Hello World\r\n".to_vec()),
    ];

    for (name, data) in test_cases {
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_function(name, |b| {
            b.iter(|| {
                let mut decoder = Resp3Decoder::new();
                let result = decoder.decode(black_box(&data)).unwrap();
                black_box(result);
            });
        });
    }

    group.finish();
}

// Command Builder Benchmarks
fn bench_command_builders(c: &mut Criterion) {
    let mut group = c.benchmark_group("command_builders");

    // Basic command creation
    group.bench_function("get_command_creation", |b| {
        b.iter(|| {
            let cmd = GetCommand::new(black_box("test_key"));
            black_box(cmd);
        });
    });

    group.bench_function("set_command_creation", |b| {
        b.iter(|| {
            let cmd = SetCommand::new(black_box("test_key"), black_box("test_value"));
            black_box(cmd);
        });
    });

    // Command argument generation
    group.bench_function("get_command_args", |b| {
        let cmd = GetCommand::new("test_key");
        b.iter(|| {
            let args = cmd.args();
            black_box(args);
        });
    });

    group.bench_function("set_command_args", |b| {
        let cmd = SetCommand::new("test_key", "test_value");
        b.iter(|| {
            let args = cmd.args();
            black_box(args);
        });
    });

    // Complex command builders
    group.bench_function("hset_command_creation", |b| {
        b.iter(|| {
            let cmd = HSetCommand::new(
                black_box("hash_key"),
                black_box("field"),
                black_box("value"),
            );
            black_box(cmd);
        });
    });

    group.bench_function("lpush_command_creation", |b| {
        b.iter(|| {
            let cmd = LPushCommand::new(
                black_box("list_key"),
                black_box(vec!["item1".to_string(), "item2".to_string()]),
            );
            black_box(cmd);
        });
    });

    group.bench_function("sadd_command_creation", |b| {
        b.iter(|| {
            let cmd = SAddCommand::new(
                black_box("set_key"),
                black_box(vec!["member1".to_string(), "member2".to_string()]),
            );
            black_box(cmd);
        });
    });

    group.bench_function("zadd_command_creation", |b| {
        b.iter(|| {
            let mut members = HashMap::new();
            members.insert("member1".to_string(), 1.0);
            members.insert("member2".to_string(), 2.0);
            let cmd = ZAddCommand::new(black_box("zset_key"), black_box(members));
            black_box(cmd);
        });
    });

    group.finish();
}

// Response Parsing Benchmarks
fn bench_response_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("response_parsing");

    // GET command response parsing
    let get_cmd = GetCommand::new("test_key");
    let responses = vec![
        ("found", RespValue::BulkString(Bytes::from("test_value"))),
        ("not_found", RespValue::Null),
    ];

    for (name, response) in responses {
        group.bench_function(&format!("get_response_{}", name), |b| {
            b.iter(|| {
                let result = get_cmd.parse_response(black_box(response.clone())).unwrap();
                black_box(result);
            });
        });
    }

    // SET command response parsing
    let set_cmd = SetCommand::new("test_key", "test_value");
    let set_responses = vec![
        ("ok", RespValue::SimpleString("OK".to_string())),
        ("nx_failed", RespValue::Null),
    ];

    for (name, response) in set_responses {
        group.bench_function(&format!("set_response_{}", name), |b| {
            b.iter(|| {
                let result = set_cmd.parse_response(black_box(response.clone())).unwrap();
                black_box(result);
            });
        });
    }

    // Array response parsing (for LRANGE, SMEMBERS, etc.)
    for size in [10, 100, 1000].iter() {
        let array_response = RespValue::Array(generate_test_array(*size));

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("array_response", size), size, |b, _| {
            b.iter(|| {
                // Simulate parsing array response to Vec<String>
                if let RespValue::Array(items) = black_box(&array_response) {
                    let mut result = Vec::new();
                    for item in items {
                        if let RespValue::BulkString(bytes) = item {
                            result.push(String::from_utf8(bytes.to_vec()).unwrap());
                        }
                    }
                    black_box(result);
                }
            });
        });
    }

    group.finish();
}

// Memory Allocation Benchmarks
fn bench_memory_allocations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocations");

    // BytesMut allocation patterns
    group.bench_function("bytesmut_new", |b| {
        b.iter(|| {
            let buf = BytesMut::new();
            black_box(buf);
        });
    });

    group.bench_function("bytesmut_with_capacity", |b| {
        b.iter(|| {
            let buf = BytesMut::with_capacity(black_box(1024));
            black_box(buf);
        });
    });

    // String allocation patterns
    for size in [10, 100, 1000, 10000].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::new("string_repeat", size), size, |b, &size| {
            b.iter(|| {
                let s = "x".repeat(black_box(size));
                black_box(s);
            });
        });

        group.bench_with_input(
            BenchmarkId::new("string_from_utf8", size),
            size,
            |b, &size| {
                let bytes = vec![b'x'; size];
                b.iter(|| {
                    let s = String::from_utf8(black_box(bytes.clone())).unwrap();
                    black_box(s);
                });
            },
        );
    }

    // Vec allocation patterns
    for size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("vec_new", size), size, |b, &size| {
            b.iter(|| {
                let mut vec = Vec::new();
                for i in 0..size {
                    vec.push(black_box(i));
                }
                black_box(vec);
            });
        });

        group.bench_with_input(
            BenchmarkId::new("vec_with_capacity", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut vec = Vec::with_capacity(size);
                    for i in 0..size {
                        vec.push(black_box(i));
                    }
                    black_box(vec);
                });
            },
        );
    }

    group.finish();
}

// Cluster Operations Benchmarks
fn bench_cluster_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cluster_operations");

    // Slot calculation benchmarks
    let test_keys = vec![
        "simple_key",
        "{user1000}.following",
        "{user1000}.followers",
        "very_long_key_name_that_might_be_used_in_practice",
        "{hash_tag_with_long_name}.some_suffix",
    ];

    for key in test_keys {
        group.bench_function(&format!("calculate_slot_{}", key.len()), |b| {
            b.iter(|| {
                let slot = redis_oxide::cluster::calculate_slot(black_box(key.as_bytes()));
                black_box(slot);
            });
        });
    }

    // Hash tag extraction benchmarks
    let hash_tag_keys = vec![
        "{user1000}.following",
        "{user1000}.followers",
        "{very_long_hash_tag_name}.suffix",
        "no_hash_tag_key",
        "{}.empty_hash_tag",
        "{single_char}.suffix",
    ];

    for key in hash_tag_keys {
        group.bench_function(&format!("hash_tag_extraction_{}", key.len()), |b| {
            b.iter(|| {
                // This benchmarks the internal hash tag extraction logic
                let slot = redis_oxide::cluster::calculate_slot(black_box(key.as_bytes()));
                black_box(slot);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_resp2_encoding,
    bench_resp2_decoding,
    bench_resp3_encoding,
    bench_resp3_decoding,
    bench_command_builders,
    bench_response_parsing,
    bench_memory_allocations,
    bench_cluster_operations
);

criterion_main!(benches);
