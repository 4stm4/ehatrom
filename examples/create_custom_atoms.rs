// Custom atoms EEPROM creation example
use ehatrom::*;

fn main() {
    println!("🔧 Creating EEPROM with custom atoms...");

    // Create a vendor info atom
    let vendor_atom = VendorInfoAtom::new(
        0x4355, // vendor_id (example: "CU" for Custom)
        0x4154, // product_id (example: "AT" for Atom)
        2,      // product_ver
        "Custom Systems",
        "Multi-Atom HAT",
        [
            0xA1, 0xB2, 0xC3, 0xD4, 0xE5, 0xF6, 0x07, 0x18, 0x29, 0x3A, 0x4B, 0x5C, 0x6D, 0x7E,
            0x8F, 0x90,
        ],
    );

    // Create GPIO map
    let gpio_atom = GpioMapAtom {
        flags: 0x0000,
        pins: [0u8; 28], // All pins as inputs
    };

    // Create custom atoms for demonstration
    let mut custom_atoms = Vec::new();

    // Custom atom 1: Configuration data
    let config_data = b"CONFIG:LED_BRIGHTNESS=80,SENSOR_RATE=100,DEBUG=1".to_vec();
    custom_atoms.push((0x81, config_data)); // Using tuple format (type, data)

    // Custom atom 2: Calibration data (simulated sensor calibration)
    let mut calibration_data = Vec::new();
    calibration_data.extend_from_slice(&42.5f32.to_le_bytes()); // Temperature offset
    calibration_data.extend_from_slice(&1.023f32.to_le_bytes()); // Voltage multiplier
    calibration_data.extend_from_slice(&(-0.15f32).to_le_bytes()); // Pressure offset
    custom_atoms.push((0x82, calibration_data)); // Using tuple format (type, data)

    // Custom atom 3: Hardware version info
    let hw_info = format!(
        "HW_VERSION={}.{}.{},PCB_REV=C,ASSEMBLY_DATE=2024-12-20",
        env!("CARGO_PKG_VERSION_MAJOR"),
        env!("CARGO_PKG_VERSION_MINOR"),
        env!("CARGO_PKG_VERSION_PATCH")
    );
    custom_atoms.push((0x83, hw_info.into_bytes())); // Using tuple format (type, data)

    // Custom atom 4: Binary data (e.g., lookup table)
    let mut lookup_table = Vec::new();
    for i in 0..32 {
        lookup_table.push((i * i) as u8); // Simple quadratic lookup table
    }
    custom_atoms.push((0x84, lookup_table)); // Using tuple format (type, data)

    // Create EEPROM structure
    let mut eeprom = Eeprom {
        header: EepromHeader::new(),
        vendor_info: vendor_atom,
        gpio_map_bank0: gpio_atom,
        dt_blob: None,
        gpio_map_bank1: None,
        custom_atoms,
    };

    // Update header with correct counts and length
    eeprom.update_header();

    // Serialize with CRC
    #[cfg(feature = "alloc")]
    let serialized = eeprom.serialize_with_crc();
    
    #[cfg(not(feature = "alloc"))]
    let serialized = {
        let mut buffer = [0u8; 1024]; // Буфер достаточного размера
        let size = eeprom.serialize_with_crc_to_slice(&mut buffer).expect("Failed to serialize EEPROM");
        &buffer[..size]
    };
    let filename = "tests/data/custom_atoms.bin";

    // Create output directory if it doesn't exist
    if std::fs::metadata("tests/data").is_err() {
        std::fs::create_dir_all("tests/data").expect("Failed to create tests/data directory");
    }

    std::fs::write(filename, &serialized).expect("Failed to write custom atoms EEPROM file");

    println!("✅ Created {} ({} bytes)", filename, serialized.len());
    println!("📊 EEPROM contains:");
    println!("   • Standard HAT header");
    println!("   • Vendor info atom");
    println!("   • GPIO map atom");
    println!("   • 4 custom atoms:");
    println!("     - 0x81: Configuration string");
    println!("     - 0x82: Sensor calibration data");
    println!("     - 0x83: Hardware version info");
    println!("     - 0x84: Lookup table (32 bytes)");

    // Verify the created file
    if Eeprom::verify_crc(&serialized) {
        println!("✅ CRC32 verification passed");
    } else {
        println!("❌ CRC32 verification failed");
    }

    println!("🎯 Use './target/release/ehatrom show {filename}' to analyze the created EEPROM");
    println!("💡 This demonstrates how to embed custom application-specific data in HAT EEPROM");
}
