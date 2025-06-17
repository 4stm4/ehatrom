use ehatrom::*;

fn main() {
    println!("📝 Creating minimal EEPROM with basic vendor info...");

    // Create a minimal vendor info atom
    let vendor_atom = VendorInfoAtom::new(
        0x5349, // vendor_id (example: "SI" for Simple)
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
        flags: 0x0000,
        pins: [0u8; 28], // All pins as inputs
    };

    // Create EEPROM structure
    let mut eeprom = Eeprom {
        header: EepromHeader::new(),
        vendor_info: vendor_atom,
        gpio_map_bank0: gpio_atom,
        dt_blob: None,
        gpio_map_bank1: None,
        custom_atoms: Vec::new(),
    };

    // Update header with correct counts and length
    eeprom.update_header();

    // Serialize with CRC
    let serialized = eeprom.serialize_with_crc();

    // Create output directory if it doesn't exist
    if std::fs::metadata("tests/data").is_err() {
        std::fs::create_dir_all("tests/data").expect("Failed to create tests/data directory");
    }

    std::fs::write("tests/data/simple.bin", &serialized).expect("Failed to write test file");

    println!(
        "✅ Created tests/data/simple.bin ({} bytes)",
        serialized.len()
    );
    println!("📊 Minimal EEPROM structure:");
    println!("   • Header + VendorInfo + GPIO Map + CRC");
    println!("   • No Device Tree blob");
    println!("   • No custom atoms");

    // Verify the created file
    if Eeprom::verify_crc(&serialized) {
        println!("✅ CRC32 verification passed");
    } else {
        println!("❌ CRC32 verification failed");
    }
}
