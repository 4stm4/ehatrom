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
    println!("📝 Creating minimal EEPROM with basic vendor info...");

    // Create a minimal vendor info atom
    let vendor_atom = VendorInfoAtom::new(
        0x4D50, // product_id (example: "MP" for MiniProduct)
        1,      // product_ver
        "Simple",
        "MinimalHAT",
        [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
            0xEE, 0xFF,
        ], // Simple UUID
    );

    // Create minimal GPIO map
    let gpio_atom = GpioMapAtom {
        flags: 0x00,
        power: 0x00,
        pins: [0u8; 28], // All pins unused
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

    // Serialize a complete, spec-compliant HAT image (per-atom CRC-16 embedded)
    #[cfg(feature = "alloc")]
    let serialized = eeprom.serialize();

    #[cfg(not(feature = "alloc"))]
    let serialized = {
        let mut buffer = [0u8; 1024]; // Буфер достаточного размера
        let size = eeprom
            .serialize_to_slice(&mut buffer)
            .expect("Failed to serialize EEPROM");
        &buffer[..size]
    };

    // Create output directory if it doesn't exist
    if std::fs::metadata("tests/data").is_err() {
        std::fs::create_dir_all("tests/data").expect("Failed to create tests/data directory");
    }

    std::fs::write("tests/data/simple.bin", &serialized).expect("Failed to write simple file");

    println!("Created tests/data/simple.bin ({} bytes)", serialized.len());

    // Verify the created file
    if Eeprom::verify(&serialized) {
        println!("✅ CRC verification passed");
    } else {
        println!("❌ CRC verification failed");
    }
}
