//  _  _       _             _  _
// | || |  ___| |_ _ __ ___ | || |
// | || |_/ __| __| '_ ` _ \| || |_
// |__   _\__ | |_| | | | | |__   _|
//   |_| |___/\__|_|_|_| |_|  |_|
//  ___   ___ _   _| | |_ _   _ _ __ ___
// / _ \ / __| | | | | __| | | | '_ ` _ \
//| (_) | (__| |_| | | |_| |_| | | | | | |
// \___/ \___|\__,_|_|\__|\__,_|_| |_| |_|
//! # ehatrom — EEPROM HAT library for Raspberry Pi HATs
//! - [Documentation (docs.rs)](https://docs.rs/ehatrom)
//! - [GitHub](https://github.com/4stm4/ocultum/tree/main/ehatrom)
//!
//! ## Custom CRC32 implementation for bare-metal compatibility
/// CRC32 polynomial (IEEE 802.3 standard)
const CRC32_POLYNOMIAL: u32 = 0xEDB88320;

/// Pre-computed CRC32 lookup table
static CRC32_TABLE: [u32; 256] = generate_crc32_table();

/// Generate CRC32 lookup table at compile time
const fn generate_crc32_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;

    while i < 256 {
        let mut crc = i as u32;
        let mut j = 0;

        while j < 8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ CRC32_POLYNOMIAL;
            } else {
                crc >>= 1;
            }
            j += 1;
        }

        table[i] = crc;
        i += 1;
    }

    table
}

/// Custom CRC32 hasher implementation
pub struct Hasher {
    crc: u32,
}

impl Hasher {
    /// Create a new CRC32 hasher
    pub fn new() -> Self {
        Self { crc: 0xFFFFFFFF }
    }

    /// Update the CRC32 with new data
    pub fn update(&mut self, data: &[u8]) {
        for &byte in data {
            let table_index = ((self.crc ^ byte as u32) & 0xFF) as usize;
            self.crc = (self.crc >> 8) ^ CRC32_TABLE[table_index];
        }
    }

    /// Finalize and return the CRC32 value
    pub fn finalize(self) -> u32 {
        self.crc ^ 0xFFFFFFFF
    }
}

impl Default for Hasher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_data() {
        let mut hasher = Hasher::new();
        hasher.update(&[]);
        assert_eq!(hasher.finalize(), 0);
    }

    #[test]
    fn test_known_values() {
        // Test "123456789" - known CRC32 value
        let mut hasher = Hasher::new();
        hasher.update(b"123456789");
        assert_eq!(hasher.finalize(), 0xCBF43926);

        // Test "hello world"
        let mut hasher = Hasher::new();
        hasher.update(b"hello world");
        assert_eq!(hasher.finalize(), 0x0D4A1185);
    }

    #[test]
    fn test_incremental_update() {
        let mut hasher1 = Hasher::new();
        hasher1.update(b"hello");
        hasher1.update(b" ");
        hasher1.update(b"world");

        let mut hasher2 = Hasher::new();
        hasher2.update(b"hello world");

        assert_eq!(hasher1.finalize(), hasher2.finalize());
    }

    #[test]
    fn test_raspberry_pi_eeprom_data() {
        // Test with actual Raspberry Pi EEPROM header-like data
        let mut hasher = Hasher::new();
        hasher.update(b"R-Pi\x01\x00\x00\x00\x10\x00\x00\x00");
        let result = hasher.finalize();

        // This should produce a consistent CRC32 value
        // The exact value depends on the input, but it should be deterministic
        // println!("CRC32 for R-Pi header: 0x{result:08X}");
        assert_ne!(result, 0); // Should not be zero for this data
    }

    #[test]
    fn test_single_byte() {
        let mut hasher = Hasher::new();
        hasher.update(&[0xFF]);
        let result = hasher.finalize();
        assert_eq!(result, 0xFF000000);
    }

    #[test]
    fn test_consistency() {
        // Test that multiple calls with same data produce same result
        let test_data = b"Raspberry Pi HAT EEPROM test data";

        let mut hasher1 = Hasher::new();
        hasher1.update(test_data);
        let result1 = hasher1.finalize();

        let mut hasher2 = Hasher::new();
        hasher2.update(test_data);
        let result2 = hasher2.finalize();

        assert_eq!(result1, result2);
    }
}
