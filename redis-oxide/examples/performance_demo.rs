//! Performance demonstration comparing original vs optimized implementations

#![allow(clippy::uninlined_format_args)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]

use bytes::BytesMut;
use redis_oxide::{
    commands::optimized::{init_string_interner, OptimizedGetCommand, OptimizedSetCommand},
    commands::{Command, GetCommand, SetCommand},
    core::value::RespValue,
    protocol::{
        resp2::{RespDecoder, RespEncoder},
        resp2_optimized::{OptimizedRespDecoder, OptimizedRespEncoder},
    },
};
use std::io::Cursor;
use std::time::{Duration, Instant};

fn main() {
    println!("Redis-Oxide Performance Optimization Demo");
    println!("=========================================");

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

    // Original encoder
    let start = Instant::now();
    for _ in 0..iterations {
        let mut buf = BytesMut::new();
        RespEncoder::encode(&test_value, &mut buf).unwrap();
    }
    let original_time = start.elapsed();

    // Optimized encoder
    let mut encoder = OptimizedRespEncoder::new();
    let start = Instant::now();
    for _ in 0..iterations {
        encoder.encode(&test_value).unwrap();
    }
    let optimized_time = start.elapsed();

    println!(
        "  Original:  {:?} ({:.2} ops/sec)",
        original_time,
        iterations as f64 / original_time.as_secs_f64()
    );
    println!(
        "  Optimized: {:?} ({:.2} ops/sec)",
        optimized_time,
        iterations as f64 / optimized_time.as_secs_f64()
    );

    let improvement =
        (original_time.as_nanos() as f64 / optimized_time.as_nanos() as f64 - 1.0) * 100.0;
    println!("  Improvement: {:.1}%", improvement);
}

fn test_resp2_decoding_performance() {
    let iterations = 10_000;
    let test_data = b"*3\r\n$3\r\nSET\r\n$5\r\nmykey\r\n$7\r\nmyvalue\r\n";

    // Original decoder
    let start = Instant::now();
    for _ in 0..iterations {
        let mut cursor = Cursor::new(&test_data[..]);
        RespDecoder::decode(&mut cursor).unwrap();
    }
    let original_time = start.elapsed();

    // Optimized decoder
    let mut decoder = OptimizedRespDecoder::new();
    let start = Instant::now();
    for _ in 0..iterations {
        decoder.decode_streaming(test_data).unwrap();
    }
    let optimized_time = start.elapsed();

    println!(
        "  Original:  {:?} ({:.2} ops/sec)",
        original_time,
        iterations as f64 / original_time.as_secs_f64()
    );
    println!(
        "  Optimized: {:?} ({:.2} ops/sec)",
        optimized_time,
        iterations as f64 / optimized_time.as_secs_f64()
    );

    let improvement =
        (original_time.as_nanos() as f64 / optimized_time.as_nanos() as f64 - 1.0) * 100.0;
    println!("  Improvement: {:.1}%", improvement);
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
    let get_time_original = start.elapsed();

    // Optimized GET command
    let start = Instant::now();
    for i in 0..iterations {
        let key = format!("key_{}", i);
        let cmd = OptimizedGetCommand::new(&key).with_cached_args();
        let _args = cmd.args();
    }
    let get_time_optimized = start.elapsed();

    println!("  GET Command:");
    println!(
        "    Original:  {:?} ({:.2} ops/sec)",
        get_time_original,
        iterations as f64 / get_time_original.as_secs_f64()
    );
    println!(
        "    Optimized: {:?} ({:.2} ops/sec)",
        get_time_optimized,
        iterations as f64 / get_time_optimized.as_secs_f64()
    );

    let get_improvement =
        (get_time_original.as_nanos() as f64 / get_time_optimized.as_nanos() as f64 - 1.0) * 100.0;
    println!("    Improvement: {:.1}%", get_improvement);

    // Original SET command
    let start = Instant::now();
    for i in 0..iterations {
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);
        let cmd = SetCommand::new(&key, &value).expire(Duration::from_secs(60));
        let _args = cmd.args();
    }
    let set_time_original = start.elapsed();

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
    let set_time_optimized = start.elapsed();

    println!("  SET Command:");
    println!(
        "    Original:  {:?} ({:.2} ops/sec)",
        set_time_original,
        iterations as f64 / set_time_original.as_secs_f64()
    );
    println!(
        "    Optimized: {:?} ({:.2} ops/sec)",
        set_time_optimized,
        iterations as f64 / set_time_optimized.as_secs_f64()
    );

    let set_improvement =
        (set_time_original.as_nanos() as f64 / set_time_optimized.as_nanos() as f64 - 1.0) * 100.0;
    println!("    Improvement: {:.1}%", set_improvement);
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
        let _s = intern_string(&format!("key_{}", i % 100));
    }
    let string_intern_time = start.elapsed();

    println!("  String Allocation:");
    println!(
        "    Regular allocation: {:?} ({:.2} ops/sec)",
        string_alloc_time,
        iterations as f64 / string_alloc_time.as_secs_f64()
    );
    println!(
        "    String interning:   {:?} ({:.2} ops/sec)",
        string_intern_time,
        iterations as f64 / string_intern_time.as_secs_f64()
    );

    let improvement =
        (string_alloc_time.as_nanos() as f64 / string_intern_time.as_nanos() as f64 - 1.0) * 100.0;
    println!("    Improvement: {:.1}%", improvement);
}

fn test_bulk_operations() {
    let batch_sizes = [10, 100, 1000];

    for &batch_size in &batch_sizes {
        println!("  Batch size: {}", batch_size);

        // Original approach
        let start = Instant::now();
        let mut total_bytes = 0;
        for i in 0..batch_size {
            let key = format!("key_{}", i);
            let value = format!("value_{}", i);
            let cmd = SetCommand::new(&key, &value);
            let args = cmd.args();
            let encoded = RespEncoder::encode_command("SET", &args).unwrap();
            total_bytes += encoded.len();
        }
        let original_time = start.elapsed();

        // Optimized approach
        let start = Instant::now();
        let mut opt_encoder = OptimizedRespEncoder::new();
        let mut total_bytes_opt = 0;
        for i in 0..batch_size {
            let key = format!("key_{}", i);
            let value = format!("value_{}", i);
            let cmd = OptimizedSetCommand::new(&key, &value).with_cached_args();
            let args = cmd.args();
            let encoded_cmd = opt_encoder.encode_command("SET", &args).unwrap();
            total_bytes_opt += encoded_cmd.len();
        }
        let optimized_time = start.elapsed();

        println!(
            "    Original:  {:?} ({} bytes, {:.2} ops/sec)",
            original_time,
            total_bytes,
            batch_size as f64 / original_time.as_secs_f64()
        );
        println!(
            "    Optimized: {:?} ({} bytes, {:.2} ops/sec)",
            optimized_time,
            total_bytes_opt,
            batch_size as f64 / optimized_time.as_secs_f64()
        );

        let improvement =
            (original_time.as_nanos() as f64 / optimized_time.as_nanos() as f64 - 1.0) * 100.0;
        println!("    Improvement: {:.1}%", improvement);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_demo_runs() {
        // Just ensure the demo runs without panicking
        init_string_interner(100);

        // Test a small subset to ensure functionality
        let test_value = RespValue::SimpleString("OK".to_string());

        // Original encoder
        let mut buf = BytesMut::new();
        RespEncoder::encode(&test_value, &mut buf).unwrap();

        // Optimized encoder
        let mut encoder = OptimizedRespEncoder::new();
        encoder.encode(&test_value).unwrap();

        println!("Performance demo test passed");
    }
}
