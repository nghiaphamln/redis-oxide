use criterion::{black_box, criterion_group, criterion_main, Criterion};
use redis_oxide::protocol::{RespDecoder, RespEncoder};
use redis_oxide::RespValue;
use bytes::{Bytes, BytesMut};
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

fn bench_encode_command(c: &mut Criterion) {
    c.bench_function("encode_command", |b| {
        let args = vec![
            RespValue::BulkString(Bytes::from("mykey")),
            RespValue::BulkString(Bytes::from("myvalue")),
        ];
        b.iter(|| {
            RespEncoder::encode_command(black_box("SET"), black_box(&args)).unwrap();
        });
    });
}

fn bench_decode_simple_string(c: &mut Criterion) {
    c.bench_function("decode_simple_string", |b| {
        let data = b"+OK\r\n";
        b.iter(|| {
            let mut cursor = Cursor::new(black_box(&data[..]));
            RespDecoder::decode(&mut cursor).unwrap();
        });
    });
}

fn bench_decode_bulk_string(c: &mut Criterion) {
    c.bench_function("decode_bulk_string", |b| {
        let data = b"$13\r\nHello, Redis!\r\n";
        b.iter(|| {
            let mut cursor = Cursor::new(black_box(&data[..]));
            RespDecoder::decode(&mut cursor).unwrap();
        });
    });
}

fn bench_decode_array(c: &mut Criterion) {
    c.bench_function("decode_array", |b| {
        let data = b"*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n";
        b.iter(|| {
            let mut cursor = Cursor::new(black_box(&data[..]));
            RespDecoder::decode(&mut cursor).unwrap();
        });
    });
}

criterion_group!(
    benches,
    bench_encode_simple_string,
    bench_encode_bulk_string,
    bench_encode_array,
    bench_encode_command,
    bench_decode_simple_string,
    bench_decode_bulk_string,
    bench_decode_array,
);
criterion_main!(benches);

