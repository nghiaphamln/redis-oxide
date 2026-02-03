//! Performance demonstration for redis-oxide
//!
//! This example requires the `internal-optimizations` feature to compile.
//! Run with: cargo run --example performance_demo --features internal-optimizations

#![cfg(feature = "internal-optimizations")]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]

use bytes::BytesMut;
use redis_oxide::{
    commands::optimized::{init_string_interner, OptimizedGetCommand, OptimizedSetCommand},
    commands::{Command, GetCommand, SetCommand},
    core::value::RespValue,
    protocol::resp2::{RespDecoder, RespEncoder},
};
use std::io::Cursor;
use std::time::{Duration, Instant};

fn main() {
    println!("Redis-Oxide Performance Demo");
    println!("==============================");

    // Initialize string interner for optimized commands
    init_string_interner(1000);

    // Test 1: RESP2 Encoding Performance
    println!("\n1. RESP2 Encoding Performance:");
    test_resp2_encoding_performance();

    // Test 2: RESP2 Decoding Performance
    println!("\n2. RESP2 Decoding Performance:");
    test_resp2_decoding_performance();

    // Test 3: Command Building Performance
    println!("\n3. Command Building Performance:");
    test_command_building_performance();

    // Test 4: Memory Allocation Patterns
    println!("\n4. Memory Allocation Patterns:");
    test_memory_allocation_patterns();

    // Test 5: Bulk Operations
    println!("\n5. Bulk Operations Performance:");
    test_bulk_operations();
}

fn test_resp2_encoding_performance() {
    let iterations = 10_000;
    let test_value = RespValue::Array(vec![
        RespValue::from("SET"),
        RespValue::from("mykey"),
        RespValue::from("myvalue"),
    ]);

    // Encoder with buffer reuse
    let mut encoder = RespEncoder::new();
    let start = Instant::now();
    for _ in 0..iterations {
        encoder.encode(&test_value).unwrap();
    }
    let time = start.elapsed();

    println!(
        "  Encode:  {:?} ({:.2} ops/sec)",
        time,
        iterations as f64 / time.as_secs_f64()
    );
}

fn test_resp2_decoding_performance() {
    let iterations = 10_000;
    let test_data = b"*3\r\n$3\r\nSET\r\n$5\r\nmykey\r\n$7\r\nmyvalue\r\n";

    let start = Instant::now();
    for _ in 0..iterations {
        let mut cursor = Cursor::new(&test_data[..]);
        RespDecoder::decode(&mut cursor).unwrap();
    }
    let time = start.elapsed();

    println!(
        "  Decode:  {:?} ({:.2} ops/sec)",
        time,
        iterations as f64 / time.as_secs_f64()
    );
}

fn test_command_building_performance() {
    let iterations = 10_000;

    // Original GET command
    let start = Instant::now();
    for i in 0..iterations {
        let key = format!("key_{}", i);
        let cmd = GetCommand::new(&key);
        let _args = cmd.args();
    }
    let get_time = start.elapsed();

    // Optimized GET command
    let start = Instant::now();
    for i in 0..iterations {
        let key = format!("key_{}", i);
        let cmd = OptimizedGetCommand::new(&key).with_cached_args();
        let _args = cmd.args();
    }
    let get_opt_time = start.elapsed();

    println!("  GET Command:");
    println!(
        "    Standard:  {:?} ({:.2} ops/sec)",
        get_time,
        iterations as f64 / get_time.as_secs_f64()
    );
    println!(
        "    Optimized: {:?} ({:.2} ops/sec)",
        get_opt_time,
        iterations as f64 / get_opt_time.as_secs_f64()
    );

    // Original SET command
    let start = Instant::now();
    for i in 0..iterations {
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);
        let cmd = SetCommand::new(&key, &value).expire(Duration::from_secs(60));
        let _args = cmd.args();
    }
    let set_time = start.elapsed();

    // Optimized SET command
    let start = Instant::now();
    for i in 0..iterations {
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);
        let cmd = OptimizedSetCommand::new(&key, &value)
            .expire(Duration::from_secs(60))
            .with_cached_args();
        let _args = cmd.args();
    }
    let set_opt_time = start.elapsed();

    println!("  SET Command:");
    println!(
        "    Standard:  {:?} ({:.2} ops/sec)",
        set_time,
        iterations as f64 / set_time.as_secs_f64()
    );
    println!(
        "    Optimized: {:?} ({:.2} ops/sec)",
        set_opt_time,
        iterations as f64 / set_opt_time.as_secs_f64()
    );
}

fn test_memory_allocation_patterns() {
    let iterations = 10_000;

    // BytesMut without pre-sizing
    let start = Instant::now();
    for _ in 0..iterations {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"hello world this is a test string");
        let _frozen = buf.freeze();
    }
    let no_presize_time = start.elapsed();

    // BytesMut with pre-sizing
    let start = Instant::now();
    for _ in 0..iterations {
        let mut buf = BytesMut::with_capacity(64);
        buf.extend_from_slice(b"hello world this is a test string");
        let _frozen = buf.freeze();
    }
    let presize_time = start.elapsed();

    println!("  BytesMut Allocation:");
    println!(
        "    Without pre-sizing: {:?} ({:.2} ops/sec)",
        no_presize_time,
        iterations as f64 / no_presize_time.as_secs_f64()
    );
    println!(
        "    With pre-sizing:    {:?} ({:.2} ops/sec)",
        presize_time,
        iterations as f64 / presize_time.as_secs_f64()
    );

    let improvement =
        (no_presize_time.as_nanos() as f64 / presize_time.as_nanos() as f64 - 1.0) * 100.0;
    println!("    Improvement: {:.1}%", improvement);

    // String allocation vs interning
    let start = Instant::now();
    for i in 0..iterations {
        let _s = format!("key_{}", i % 100); // Simulate repeated keys
    }
    let string_alloc_time = start.elapsed();

    let start = Instant::now();
    for i in 0..iterations {
        use redis_oxide::commands::optimized::intern_string;
        let key = format!("key_{}", i % 100);
        let _s = intern_string(&key);
    }
    let intern_time = start.elapsed();

    println!("  String Allocation:");
    println!(
        "    Standard:  {:?} ({:.2} ops/sec)",
        string_alloc_time,
        iterations as f64 / string_alloc_time.as_secs_f64()
    );
    println!(
        "    Interned:  {:?} ({:.2} ops/sec)",
        intern_time,
        iterations as f64 / intern_time.as_secs_f64()
    );

    let improvement =
        (string_alloc_time.as_nanos() as f64 / intern_time.as_nanos() as f64 - 1.0) * 100.0;
    println!("    Improvement: {:.1}%", improvement);
}

fn test_bulk_operations() {
    let iterations = 1000;
    let batch_size = 100;

    // Test batch command building with optimized commands
    let start = Instant::now();
    for batch in (0..iterations).step_by(batch_size) {
        let mut commands = Vec::with_capacity(batch_size);
        for i in batch..(batch + batch_size).min(iterations) {
            let key = format!("key_{}", i);
            let value = format!("value_{}", i);
            let cmd = OptimizedSetCommand::new(&key, &value)
                .expire(Duration::from_secs(3600))
                .with_cached_args();
            commands.push(cmd);
        }
    }
    let time = start.elapsed();

    println!(
        "  Batch Build ({}x{}): {:?} ({:.2} ops/sec)",
        iterations / batch_size,
        batch_size,
        time,
        iterations as f64 / time.as_secs_f64()
    );
}
