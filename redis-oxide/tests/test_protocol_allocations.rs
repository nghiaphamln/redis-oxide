//! Allocation reporting for protocol hot paths.

use bytes::{Bytes, BytesMut};
use redis_oxide::protocol::{Resp3Decoder, Resp3Encoder, RespDecoder, RespEncoder};
use redis_oxide::{Resp3Value, RespValue};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;
use std::hint::black_box;
use std::io::Cursor;

const ITERATIONS: usize = 1_000;
const RESP2_SIMPLE: &[u8] = b"+OK\r\n";
const RESP2_BULK: &[u8] = b"$13\r\nHello, Redis!\r\n";
const RESP2_ARRAY: &[u8] = b"*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n";
const RESP3_SIMPLE: &[u8] = b"+OK\r\n";
const RESP3_BLOB: &[u8] = b"$13\r\nHello, Redis!\r\n";
const RESP3_ARRAY: &[u8] = b"*3\r\n$3\r\nSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n";

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn report_operation<F>(operation: &str, mut action: F)
where
    F: FnMut(),
{
    action();

    let region = Region::new(GLOBAL);
    for _ in 0..ITERATIONS {
        action();
    }
    let stats = region.change();

    println!(
        "allocation_report,{operation},{ITERATIONS},{}/{},{}/{},{}/{}",
        stats.allocations,
        ITERATIONS,
        stats.bytes_allocated,
        ITERATIONS,
        stats.reallocations,
        ITERATIONS,
    );
}

#[test]
#[ignore = "reports allocation metrics; run manually"]
fn protocol_allocation_report() {
    println!(
        "allocation_report,operation,iterations,allocations_per_op,bytes_allocated_per_op,reallocations_per_op"
    );

    let resp2_simple = RespValue::SimpleString("OK".to_string());
    report_operation("resp2_encode_simple_string", || {
        let mut buffer = BytesMut::new();
        RespEncoder::encode(black_box(&resp2_simple), &mut buffer).unwrap();
        assert_eq!(&buffer[..], RESP2_SIMPLE);
        black_box(buffer);
    });

    let resp2_bulk = RespValue::BulkString(Bytes::from_static(b"Hello, Redis!"));
    report_operation("resp2_encode_bulk_string", || {
        let mut buffer = BytesMut::new();
        RespEncoder::encode(black_box(&resp2_bulk), &mut buffer).unwrap();
        assert_eq!(&buffer[..], RESP2_BULK);
        black_box(buffer);
    });

    let resp2_array = RespValue::Array(vec![
        RespValue::BulkString(Bytes::from_static(b"SET")),
        RespValue::BulkString(Bytes::from_static(b"key")),
        RespValue::BulkString(Bytes::from_static(b"value")),
    ]);
    report_operation("resp2_encode_array", || {
        let mut buffer = BytesMut::new();
        RespEncoder::encode(black_box(&resp2_array), &mut buffer).unwrap();
        assert_eq!(&buffer[..], RESP2_ARRAY);
        black_box(buffer);
    });

    let resp2_simple_expected = RespValue::SimpleString("OK".to_string());
    report_operation("resp2_decode_simple_string", || {
        let mut cursor = Cursor::new(black_box(RESP2_SIMPLE));
        let value = RespDecoder::decode(&mut cursor).unwrap().unwrap();
        assert_eq!(value, resp2_simple_expected);
        black_box(value);
    });

    let resp2_bulk_expected = RespValue::BulkString(Bytes::from_static(b"Hello, Redis!"));
    report_operation("resp2_decode_bulk_string", || {
        let mut cursor = Cursor::new(black_box(RESP2_BULK));
        let value = RespDecoder::decode(&mut cursor).unwrap().unwrap();
        assert_eq!(value, resp2_bulk_expected);
        black_box(value);
    });

    let resp2_array_expected = RespValue::Array(vec![
        RespValue::BulkString(Bytes::from_static(b"SET")),
        RespValue::BulkString(Bytes::from_static(b"key")),
        RespValue::BulkString(Bytes::from_static(b"value")),
    ]);
    report_operation("resp2_decode_array", || {
        let mut cursor = Cursor::new(black_box(RESP2_ARRAY));
        let value = RespDecoder::decode(&mut cursor).unwrap().unwrap();
        assert_eq!(value, resp2_array_expected);
        black_box(value);
    });

    let resp3_simple = Resp3Value::SimpleString("OK".to_string());
    let mut resp3_encoder = Resp3Encoder::new();
    report_operation("resp3_encode_simple_string", || {
        let encoded = resp3_encoder.encode(black_box(&resp3_simple)).unwrap();
        assert_eq!(&encoded[..], RESP3_SIMPLE);
        black_box(encoded);
    });

    let resp3_blob = Resp3Value::BlobString(Bytes::from_static(b"Hello, Redis!"));
    report_operation("resp3_encode_blob_string", || {
        let encoded = resp3_encoder.encode(black_box(&resp3_blob)).unwrap();
        assert_eq!(&encoded[..], RESP3_BLOB);
        black_box(encoded);
    });

    let resp3_array = Resp3Value::Array(vec![
        Resp3Value::BlobString(Bytes::from_static(b"SET")),
        Resp3Value::BlobString(Bytes::from_static(b"key")),
        Resp3Value::BlobString(Bytes::from_static(b"value")),
    ]);
    report_operation("resp3_encode_array", || {
        let encoded = resp3_encoder.encode(black_box(&resp3_array)).unwrap();
        assert_eq!(&encoded[..], RESP3_ARRAY);
        black_box(encoded);
    });

    let resp3_simple_expected = Resp3Value::SimpleString("OK".to_string());
    let mut resp3_decoder = Resp3Decoder::new();
    report_operation("resp3_decode_simple_string", || {
        let value = resp3_decoder.decode(black_box(RESP3_SIMPLE)).unwrap();
        assert_eq!(value, resp3_simple_expected);
        black_box(value);
    });

    let resp3_blob_expected = Resp3Value::BlobString(Bytes::from_static(b"Hello, Redis!"));
    report_operation("resp3_decode_blob_string", || {
        let value = resp3_decoder.decode(black_box(RESP3_BLOB)).unwrap();
        assert_eq!(value, resp3_blob_expected);
        black_box(value);
    });

    let resp3_array_expected = Resp3Value::Array(vec![
        Resp3Value::BlobString(Bytes::from_static(b"SET")),
        Resp3Value::BlobString(Bytes::from_static(b"key")),
        Resp3Value::BlobString(Bytes::from_static(b"value")),
    ]);
    report_operation("resp3_decode_array", || {
        let value = resp3_decoder.decode(black_box(RESP3_ARRAY)).unwrap();
        assert_eq!(value, resp3_array_expected);
        black_box(value);
    });
}
