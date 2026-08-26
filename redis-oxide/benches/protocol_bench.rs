#![allow(missing_docs)]

use bytes::{Bytes, BytesMut};
use criterion::{criterion_group, criterion_main, Criterion};
use redis_oxide::protocol::{Resp3Decoder, Resp3Encoder, RespDecoder, RespEncoder};
use redis_oxide::{Resp3Value, RespValue};
use std::hint::black_box;
use std::io::Cursor;

fn bench_encode_simple_string(c: &mut Criterion) {
    c.bench_function("encode_simple_string", |b| {
        let value = RespValue::SimpleString("OK".to_string());
        b.iter(|| {
            let mut buf = BytesMut::new();
            RespEncoder::encode(black_box(&value), &mut buf).unwrap();
            black_box(buf);
        });
    });
}

fn bench_encode_bulk_string(c: &mut Criterion) {
    c.bench_function("encode_bulk_string", |b| {
        let value = RespValue::BulkString(Bytes::from("Hello, Redis!"));
        b.iter(|| {
            let mut buf = BytesMut::new();
            RespEncoder::encode(black_box(&value), &mut buf).unwrap();
            black_box(buf);
        });
    });
}

fn bench_encode_array(c: &mut Criterion) {
    c.bench_function("encode_array", |b| {
        let value = RespValue::Array(vec![
            RespValue::BulkString(Bytes::from("SET")),
            RespValue::BulkString(Bytes::from("key")),
            RespValue::BulkString(Bytes::from("value")),
        ]);
        b.iter(|| {
            let mut buf = BytesMut::new();
            RespEncoder::encode(black_box(&value), &mut buf).unwrap();
            black_box(buf);
        });
    });
}

fn bench_encode_simple_string_reuse_buffer(c: &mut Criterion) {
    c.bench_function("encode_simple_string_reuse_buffer", |b| {
        let value = RespValue::SimpleString("OK".to_string());
        let mut buf = BytesMut::with_capacity(8);
        b.iter(|| {
            buf.clear();
            RespEncoder::encode(black_box(&value), &mut buf).unwrap();
            black_box(&buf);
        });
    });
}

fn bench_encode_bulk_string_reuse_buffer(c: &mut Criterion) {
    c.bench_function("encode_bulk_string_reuse_buffer", |b| {
        let value = RespValue::BulkString(Bytes::from("Hello, Redis!"));
        let mut buf = BytesMut::with_capacity(32);
        b.iter(|| {
            buf.clear();
            RespEncoder::encode(black_box(&value), &mut buf).unwrap();
            black_box(&buf);
        });
    });
}

fn bench_encode_array_reuse_buffer(c: &mut Criterion) {
    c.bench_function("encode_array_reuse_buffer", |b| {
        let value = RespValue::Array(vec![
            RespValue::BulkString(Bytes::from("SET")),
            RespValue::BulkString(Bytes::from("key")),
            RespValue::BulkString(Bytes::from("value")),
        ]);
        let mut buf = BytesMut::with_capacity(40);
        b.iter(|| {
            buf.clear();
            RespEncoder::encode(black_box(&value), &mut buf).unwrap();
            black_box(&buf);
        });
    });
}

fn bench_encode_command(c: &mut Criterion) {
    c.bench_function("encode_command", |b| {
        let args = vec![
            RespValue::BulkString(Bytes::from("mykey")),
            RespValue::BulkString(Bytes::from("myvalue")),
        ];
        b.iter(|| {
            let command = RespEncoder::encode_command(black_box("SET"), black_box(&args)).unwrap();
            black_box(command);
        });
    });
}

fn bench_decode_simple_string(c: &mut Criterion) {
    c.bench_function("decode_simple_string", |b| {
        let data = b"+OK\r\n";
        b.iter(|| {
            let mut cursor = Cursor::new(black_box(&data[..]));
            let value = RespDecoder::decode(&mut cursor).unwrap();
            black_box(value);
        });
    });
}

fn bench_decode_bulk_string(c: &mut Criterion) {
    c.bench_function("decode_bulk_string", |b| {
        let data = b"$13\r\nHello, Redis!\r\n";
        b.iter(|| {
            let mut cursor = Cursor::new(black_box(&data[..]));
            let value = RespDecoder::decode(&mut cursor).unwrap();
            black_box(value);
        });
    });
}

fn bench_decode_array(c: &mut Criterion) {
    c.bench_function("decode_array", |b| {
        let data = b"*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n";
        b.iter(|| {
            let mut cursor = Cursor::new(black_box(&data[..]));
            let value = RespDecoder::decode(&mut cursor).unwrap();
            black_box(value);
        });
    });
}

fn bench_resp3_encode_simple_string(c: &mut Criterion) {
    c.bench_function("resp3_encode_simple_string", |b| {
        let value = Resp3Value::SimpleString("OK".to_string());
        let mut encoder = Resp3Encoder::new();
        b.iter(|| {
            let output = encoder.encode(black_box(&value)).unwrap();
            black_box(output);
        });
    });
}

fn bench_resp3_encode_blob_string(c: &mut Criterion) {
    c.bench_function("resp3_encode_blob_string", |b| {
        let value = Resp3Value::BlobString(Bytes::from("Hello, Redis!"));
        let mut encoder = Resp3Encoder::new();
        b.iter(|| {
            let output = encoder.encode(black_box(&value)).unwrap();
            black_box(output);
        });
    });
}

fn bench_resp3_encode_array(c: &mut Criterion) {
    c.bench_function("resp3_encode_array", |b| {
        let value = Resp3Value::Array(vec![
            Resp3Value::BlobString(Bytes::from("SET")),
            Resp3Value::BlobString(Bytes::from("key")),
            Resp3Value::BlobString(Bytes::from("value")),
        ]);
        let mut encoder = Resp3Encoder::new();
        b.iter(|| {
            let output = encoder.encode(black_box(&value)).unwrap();
            black_box(output);
        });
    });
}

fn bench_resp3_decode_simple_string(c: &mut Criterion) {
    c.bench_function("resp3_decode_simple_string", |b| {
        let data = b"+OK\r\n";
        let mut decoder = Resp3Decoder::new();
        b.iter(|| {
            let value = decoder.decode(black_box(&data[..])).unwrap();
            black_box(value);
        });
    });
}

fn bench_resp3_decode_blob_string(c: &mut Criterion) {
    c.bench_function("resp3_decode_blob_string", |b| {
        let data = b"$13\r\nHello, Redis!\r\n";
        let mut decoder = Resp3Decoder::new();
        b.iter(|| {
            let value = decoder.decode(black_box(&data[..])).unwrap();
            black_box(value);
        });
    });
}

fn bench_resp3_decode_array(c: &mut Criterion) {
    c.bench_function("resp3_decode_array", |b| {
        let data = b"*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n";
        let mut decoder = Resp3Decoder::new();
        b.iter(|| {
            let value = decoder.decode(black_box(&data[..])).unwrap();
            black_box(value);
        });
    });
}

criterion_group!(
    benches,
    bench_encode_simple_string,
    bench_encode_bulk_string,
    bench_encode_array,
    bench_encode_simple_string_reuse_buffer,
    bench_encode_bulk_string_reuse_buffer,
    bench_encode_array_reuse_buffer,
    bench_encode_command,
    bench_decode_simple_string,
    bench_decode_bulk_string,
    bench_decode_array,
    bench_resp3_encode_simple_string,
    bench_resp3_encode_blob_string,
    bench_resp3_encode_array,
    bench_resp3_decode_simple_string,
    bench_resp3_decode_blob_string,
    bench_resp3_decode_array,
);
criterion_main!(benches);
