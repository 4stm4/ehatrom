//  _  _       _             _  _
// | || |  ___| |_ _ __ ___ | || |
// | || |_/ __| __| '_ ` _ \| || |_
// |__   _\__ | |_| | | | | |__   _|
//   |_| |___/\__|_|_|_| |_|  |_|
//! # ehatrom — EEPROM HAT library for Raspberry Pi HATs
//! - [Documentation (docs.rs)](https://docs.rs/ehatrom)
//! - [GitHub](https://github.com/4stm4/ehatrom)
//!
use ehatrom::*;

fn main() {
    // Create a vendor info atom
    let vendor_atom = VendorInfoAtom::new(
        0x4D4F, // vendor_id (example: "MO")
        0x1234, // product_id
        1,      // product_ver
        "TestVendor",
        "TestProduct",
        [
            0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC,
            0xDE, 0xF0,
        ], // UUID
    );

    // Create GPIO map for bank 0
    let gpio_atom = GpioMapAtom {
        flags: 0x0000,
        pins: [0u8; 28], // All pins as inputs
    };

    // Create EEPROM structure
    #[cfg(feature = "alloc")]
    let mut eeprom = Eeprom {
        header: EepromHeader::new(),
        vendor_info: vendor_atom,
        gpio_map_bank0: gpio_atom,
        dt_blob: None,
        gpio_map_bank1: None,
        custom_atoms: Vec::new(),
    };

    #[cfg(not(feature = "alloc"))]
    let mut eeprom = Eeprom {
        header: EepromHeader::new(),
        vendor_info: vendor_atom,
        gpio_map_bank0: gpio_atom,
        dt_blob: None,
        gpio_map_bank1: None,
        custom_atoms: &[],
    };

    // Update header with correct counts and length
    eeprom.update_header();

    // Serialize with CRC
    #[cfg(feature = "alloc")]
    let serialized = eeprom.serialize_with_crc();

    #[cfg(not(feature = "alloc"))]
    let serialized = {
        let mut buffer = [0u8; 1024]; // Буфер достаточного размера
        let size = eeprom
            .serialize_with_crc_to_slice(&mut buffer)
            .expect("Failed to serialize EEPROM");
        &buffer[..size]
    };

    // Create output directory if it doesn't exist
    if std::fs::metadata("tests/data").is_err() {
        std::fs::create_dir_all("tests/data").expect("Failed to create tests/data directory");
    }

    std::fs::write("tests/data/test.bin", &serialized).expect("Failed to write test file");

    println!("Created tests/data/test.bin ({} bytes)", serialized.len());

    // Verify the created file
    if Eeprom::verify_crc(&serialized) {
        println!("✅ CRC verification passed");
    } else {
        println!("❌ CRC verification failed");
    }
}
