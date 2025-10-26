//! Benchmark comparing original vs optimized implementations

#![allow(missing_docs)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::explicit_iter_loop)]
#![allow(clippy::similar_names)]

use bytes::{Bytes, BytesMut};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use redis_oxide::{
    commands::optimized::{init_string_interner, OptimizedGetCommand, OptimizedSetCommand},
    commands::{Command, GetCommand, SetCommand},
    core::value::RespValue,
    protocol::{
        resp2::{RespDecoder, RespEncoder},
        resp2_optimized::{OptimizedRespDecoder, OptimizedRespEncoder},
    },
};
use std::hint::black_box;
use std::io::Cursor;
use std::time::Duration;

fn setup_string_interner() {
    init_string_interner(1000);
}

fn bench_resp2_encoding_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("resp2_encoding_comparison");

    // Test data
    let test_values = vec![
        ("simple_string", RespValue::SimpleString("OK".to_string())),
        (
            "bulk_string",
            RespValue::BulkString(Bytes::from("hello world")),
        ),
        ("integer", RespValue::Integer(42)),
        (
            "array",
            RespValue::Array(vec![
                RespValue::from("item1"),
                RespValue::from("item2"),
                RespValue::from("item3"),
            ]),
        ),
    ];

    for (name, value) in test_values {
        // Original encoder
        group.bench_function(&format!("original_{}", name), |b| {
            b.iter(|| {
                let mut buf = BytesMut::new();
                RespEncoder::encode(black_box(&value), &mut buf).unwrap();
                black_box(buf);
            });
        });

        // Optimized encoder
        group.bench_function(&format!("optimized_{}", name), |b| {
            let mut opt_encoder = OptimizedRespEncoder::new();
            b.iter(|| {
                let result = opt_encoder.encode(black_box(&value)).unwrap();
                black_box(result);
            });
        });
    }

    group.finish();
}

fn bench_resp2_decoding_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("resp2_decoding_comparison");

    let test_data = vec![
        ("simple_string", b"+OK\r\n".to_vec()),
        ("bulk_string", b"$11\r\nhello world\r\n".to_vec()),
        ("integer", b":42\r\n".to_vec()),
        (
            "array",
            b"*3\r\n$5\r\nitem1\r\n$5\r\nitem2\r\n$5\r\nitem3\r\n".to_vec(),
        ),
    ];

    for (name, data) in test_data {
        // Original decoder
        group.bench_function(&format!("original_{}", name), |b| {
            b.iter(|| {
                let mut cursor = Cursor::new(black_box(&data[..]));
                let result = RespDecoder::decode(&mut cursor).unwrap();
                black_box(result);
            });
        });

        // Optimized decoder (streaming)
        group.bench_function(&format!("optimized_{}", name), |b| {
            let mut decoder = OptimizedRespDecoder::new();
            b.iter(|| {
                let results = decoder.decode_streaming(black_box(&data)).unwrap();
                black_box(results);
            });
        });
    }

    group.finish();
}

fn bench_command_encoding_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("command_encoding_comparison");

    // Original GET command
    group.bench_function("original_get_command", |b| {
        let cmd = GetCommand::new("test_key");
        b.iter(|| {
            let args = cmd.args();
            let encoded = RespEncoder::encode_command(black_box("GET"), black_box(&args)).unwrap();
            black_box(encoded);
        });
    });

    // Optimized GET command
    group.bench_function("optimized_get_command", |b| {
        setup_string_interner();
        let cmd = OptimizedGetCommand::new("test_key").with_cached_args();
        let mut opt_encoder = OptimizedRespEncoder::new();
        b.iter(|| {
            let args = cmd.args();
            let result = opt_encoder
                .encode_command(black_box("GET"), black_box(&args))
                .unwrap();
            black_box(result);
        });
    });

    // Original SET command
    group.bench_function("original_set_command", |b| {
        let cmd = SetCommand::new("test_key", "test_value").expire(Duration::from_secs(60));
        b.iter(|| {
            let args = cmd.args();
            let set_encoded =
                RespEncoder::encode_command(black_box("SET"), black_box(&args)).unwrap();
            black_box(set_encoded);
        });
    });

    // Optimized SET command
    group.bench_function("optimized_set_command", |b| {
        setup_string_interner();
        let cmd = OptimizedSetCommand::new("test_key", "test_value")
            .expire(Duration::from_secs(60))
            .with_cached_args();
        let mut opt_encoder2 = OptimizedRespEncoder::new();
        b.iter(|| {
            let args = cmd.args();
            let set_encoded = opt_encoder2
                .encode_command(black_box("SET"), black_box(&args))
                .unwrap();
            black_box(set_encoded);
        });
    });

    group.finish();
}

fn bench_memory_allocation_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation_patterns");

    // BytesMut allocation comparison
    group.bench_function("bytesmut_new_each_time", |b| {
        b.iter(|| {
            let mut buf = BytesMut::new();
            buf.extend_from_slice(b"hello world");
            black_box(buf);
        });
    });

    group.bench_function("bytesmut_with_capacity", |b| {
        b.iter(|| {
            let mut buf = BytesMut::with_capacity(64);
            buf.extend_from_slice(b"hello world");
            black_box(buf);
        });
    });

    // String allocation comparison
    group.bench_function("string_allocation_each_time", |b| {
        b.iter(|| {
            let s = format!("key_{}", black_box(42));
            black_box(s);
        });
    });

    group.bench_function("string_interning", |b| {
        setup_string_interner();
        b.iter(|| {
            use redis_oxide::commands::optimized::intern_string;
            let s = intern_string(&format!("key_{}", black_box(42)));
            black_box(s);
        });
    });

    group.finish();
}

fn bench_bulk_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("bulk_operations");

    // Bulk encoding - original
    for size in [10, 100, 1000].iter() {
        let commands: Vec<_> = (0..*size)
            .map(|i| {
                let key = format!("key_{}", i);
                let value = format!("value_{}", i);
                (key, value)
            })
            .collect();

        group.bench_with_input(BenchmarkId::new("original_bulk_set", size), size, |b, _| {
            b.iter(|| {
                let mut total_size = 0;
                for (key, value) in &commands {
                    let cmd = SetCommand::new(key, value);
                    let args = cmd.args();
                    let encoded = RespEncoder::encode_command("SET", &args).unwrap();
                    total_size += encoded.len();
                }
                black_box(total_size);
            });
        });

        group.bench_with_input(
            BenchmarkId::new("optimized_bulk_set", size),
            size,
            |b, _| {
                setup_string_interner();
                let optimized_commands: Vec<_> = commands
                    .iter()
                    .map(|(key, value)| OptimizedSetCommand::new(key, value).with_cached_args())
                    .collect();

                b.iter(|| {
                    let mut encoder = OptimizedRespEncoder::new();
                    let mut total_size = 0;
                    for cmd in &optimized_commands {
                        let args = cmd.args();
                        let encoded = encoder.encode_command("SET", &args).unwrap();
                        total_size += encoded.len();
                    }
                    black_box(total_size);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_resp2_encoding_comparison,
    bench_resp2_decoding_comparison,
    bench_command_encoding_comparison,
    bench_memory_allocation_patterns,
    bench_bulk_operations
);

criterion_main!(benches);
