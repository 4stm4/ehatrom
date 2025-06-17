#![cfg(feature = "std")]

use ehatrom::utils::crc32::Hasher;
use std::time::Instant;

#[test]
fn test_crc32_performance() {
    // Test data
    let data = b"This is a test string for performance measurement of our custom CRC32 implementation. Let's make it a bit longer to get more meaningful results.";
    let large_data = data.repeat(1000); // ~145KB

    println!("Testing CRC32 performance:");
    println!("Data size: {} bytes", large_data.len());

    // Warm up
    for _ in 0..10 {
        let mut hasher = Hasher::new();
        hasher.update(&large_data);
        let _ = hasher.finalize();
    }

    // Actual test
    let start = Instant::now();
    let iterations = 100;

    for _ in 0..iterations {
        let mut hasher = Hasher::new();
        hasher.update(&large_data);
        let _ = hasher.finalize();
    }

    let duration = start.elapsed();
    let throughput =
        (large_data.len() as f64 * iterations as f64) / duration.as_secs_f64() / (1024.0 * 1024.0);

    println!("Time: {duration:?} for {iterations} iterations");
    println!("Throughput: {throughput:.2} MB/s");

    // Performance assertion - should process at least 50 MB/s (default)
    // Allow override via env or lower for ARM
    let min_mbps = std::env::var("CRC32_PERF_MIN_MBPS")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(
            if cfg!(target_arch = "arm") || cfg!(target_arch = "aarch64") {
                10.0
            } else {
                50.0
            },
        );
    assert!(
        throughput > min_mbps,
        "CRC32 performance too slow: {throughput:.2} MB/s (min required: {min_mbps} MB/s)"
    );
}

#[test]
fn test_crc32_correctness() {
    // Test correctness with known value
    let mut hasher = Hasher::new();
    hasher.update(b"123456789");
    let result = hasher.finalize();

    assert_eq!(
        result, 0xCBF43926,
        "CRC32('123456789') = 0x{result:08X} (expected: 0xCBF43926)"
    );
}
