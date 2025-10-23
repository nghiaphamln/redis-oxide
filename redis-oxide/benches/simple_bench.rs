//! Simple benchmark for testing baseline performance

use bytes::{Bytes, BytesMut};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use redis_oxide::{
    core::value::RespValue,
    protocol::resp2::{RespDecoder, RespEncoder},
};
use std::io::Cursor;

fn bench_resp2_simple_encoding(c: &mut Criterion) {
    c.bench_function("resp2_encode_simple_string", |b| {
        let value = RespValue::SimpleString("OK".to_string());
        b.iter(|| {
            let mut buf = BytesMut::new();
            RespEncoder::encode(black_box(&value), &mut buf).unwrap();
            black_box(buf);
        });
    });

    c.bench_function("resp2_encode_bulk_string", |b| {
        let value = RespValue::BulkString(Bytes::from("hello world"));
        b.iter(|| {
            let mut buf = BytesMut::new();
            RespEncoder::encode(black_box(&value), &mut buf).unwrap();
            black_box(buf);
        });
    });

    c.bench_function("resp2_encode_integer", |b| {
        let value = RespValue::Integer(42);
        b.iter(|| {
            let mut buf = BytesMut::new();
            RespEncoder::encode(black_box(&value), &mut buf).unwrap();
            black_box(buf);
        });
    });
}

fn bench_resp2_simple_decoding(c: &mut Criterion) {
    c.bench_function("resp2_decode_simple_string", |b| {
        let data = b"+OK\r\n";
        b.iter(|| {
            let mut cursor = Cursor::new(black_box(&data[..]));
            let result = RespDecoder::decode(&mut cursor).unwrap();
            black_box(result);
        });
    });

    c.bench_function("resp2_decode_bulk_string", |b| {
        let data = b"$11\r\nhello world\r\n";
        b.iter(|| {
            let mut cursor = Cursor::new(black_box(&data[..]));
            let result = RespDecoder::decode(&mut cursor).unwrap();
            black_box(result);
        });
    });

    c.bench_function("resp2_decode_integer", |b| {
        let data = b":42\r\n";
        b.iter(|| {
            let mut cursor = Cursor::new(black_box(&data[..]));
            let result = RespDecoder::decode(&mut cursor).unwrap();
            black_box(result);
        });
    });
}

fn bench_command_encoding(c: &mut Criterion) {
    c.bench_function("resp2_encode_get_command", |b| {
        let args = vec![RespValue::from("mykey")];
        b.iter(|| {
            let encoded = RespEncoder::encode_command(black_box("GET"), black_box(&args)).unwrap();
            black_box(encoded);
        });
    });

    c.bench_function("resp2_encode_set_command", |b| {
        let args = vec![RespValue::from("mykey"), RespValue::from("myvalue")];
        b.iter(|| {
            let encoded = RespEncoder::encode_command(black_box("SET"), black_box(&args)).unwrap();
            black_box(encoded);
        });
    });
}

criterion_group!(
    benches,
    bench_resp2_simple_encoding,
    bench_resp2_simple_decoding,
    bench_command_encoding
);
criterion_main!(benches);
