//  _  _       _             _  _
// | || |  ___| |_ _ __ ___ | || |
// | || |_/ __| __| '_ ` _ \| || |_
// |__   _\__ | |_| | | | | |__   _|
//   |_| |___/\__|_|_|_| |_|  |_|
//! # ehatrom — EEPROM HAT library for Raspberry Pi HATs
//! - [Documentation (docs.rs)](https://docs.rs/ehatrom)
//! - [GitHub](https://github.com/4stm4/ehatrom)
//!
// Example of using ehatrom in no_std environment
// Note: This example can be compiled with std for demonstration,
// but shows how to use the library in no_std context

use ehatrom::{Eeprom, EepromHeader, GpioMapAtom, VendorInfoAtom};

fn main() {
    // This example demonstrates how to use ehatrom in a bare-metal environment
    // without heap allocation

    // Create EEPROM structure with static data
    let vendor_info = VendorInfoAtom::new(
        0x0001, // vendor_id
        0x0002, // product_id
        0x0001, // product_ver
        "Acme Corp",
        "Test HAT",
        [
            0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66,
            0x77, 0x88,
        ],
    );

    let gpio_map = GpioMapAtom {
        flags: 0x0001,
        pins: [0; 28], // All pins disabled
    };

    // Custom atoms with static data
    static CUSTOM_DATA: &[u8] = b"Hello, Bare Metal!";

    // In no_std environment, custom_atoms would be a static slice
    #[cfg(not(feature = "alloc"))]
    static CUSTOM_ATOMS: &[(u8, &[u8])] = &[(0x80, CUSTOM_DATA)];

    #[cfg(feature = "alloc")]
    let custom_atoms = vec![(0x80u8, CUSTOM_DATA.to_vec())];

    let mut eeprom = Eeprom {
        header: EepromHeader::new(),
        vendor_info,
        gpio_map_bank0: gpio_map,
        dt_blob: None,
        gpio_map_bank1: None,
        #[cfg(feature = "alloc")]
        custom_atoms,
        #[cfg(not(feature = "alloc"))]
        custom_atoms: CUSTOM_ATOMS,
    };

    eeprom.update_header();

    // Calculate required buffer size
    let buffer_size = eeprom.calculate_serialized_size();
    println!("Required buffer size: {buffer_size} bytes");

    // Serialize to fixed buffer (demonstrates no heap allocation approach)
    let mut buffer = vec![0u8; 512]; // In real no_std, this would be a static array

    #[cfg(feature = "alloc")]
    {
        // Standard allocation-based serialization
        let serialized = eeprom.serialize();
        println!("Serialized {} bytes using Vec", serialized.len());
    }

    // Demonstrate no-allocation serialization (available in both std and no_std)
    let mut offset = 0;
    match eeprom.serialize_to_buffer(&mut buffer, &mut offset) {
        Ok(()) => {
            println!("Successfully serialized {offset} bytes to buffer");
            println!("First 16 bytes: {:02X?}", &buffer[..16.min(offset)]);
        }
        Err(e) => {
            println!("Serialization failed: {e:?}");
        }
    }

    println!("EEPROM structure:\n{eeprom}");
}
