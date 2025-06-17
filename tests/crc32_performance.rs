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

    // Actual test - reduced iterations for CI
    let start = Instant::now();
    let iterations = 10; // Reduced for test environment

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

    // Performance should be reasonable (at least 1 MB/s)
    assert!(
        throughput > 1.0,
        "CRC32 performance too slow: {throughput:.2} MB/s"
    );
}

#[test]
fn test_crc32_correctness_extended() {
    // Test correctness with known value
    let mut hasher = Hasher::new();
    hasher.update(b"123456789");
    let result = hasher.finalize();

    assert_eq!(
        result, 0xCBF43926,
        "CRC32('123456789') = 0x{result:08X} (expected: 0xCBF43926)"
    );

    // Test empty string
    let mut hasher = Hasher::new();
    hasher.update(&[]);
    assert_eq!(hasher.finalize(), 0);

    // Test single byte
    let mut hasher = Hasher::new();
    hasher.update(&[0xFF]);
    let result = hasher.finalize();
    println!("CRC32([0xFF]) = 0x{result:08X}");

    // Test incremental vs all-at-once
    let mut hasher1 = Hasher::new();
    hasher1.update(b"hello");
    hasher1.update(b" ");
    hasher1.update(b"world");

    let mut hasher2 = Hasher::new();
    hasher2.update(b"hello world");

    assert_eq!(hasher1.finalize(), hasher2.finalize());
}
