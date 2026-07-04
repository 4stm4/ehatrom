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
    println!("📝 Creating EEPROM with custom atoms...");

    // Create a list to hold our custom atoms
    let mut custom_atoms = Vec::new();

    // Create vendor info atom
    let vendor_atom = VendorInfoAtom::new(
        0x0001, // product_id
        2,      // product_ver
        "ACME Custom HATs",
        "SensorBoard Plus",
        [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD,
            0xEE, 0xFF,
        ], // UUID
    );

    // Create GPIO map for bank 0 - more complex pin assignments
    // Configure pins with the typed helper. Unlisted pins stay UNUSED_PIN (0x00);
    // a configured pin always has the "board uses this pin" bit set.
    let mut pins = [UNUSED_PIN; 28];
    pins[4] = encode_pin(PinFunc::Input, PinPull::Up); // GPIO4 - input, pull-up
    pins[17] = encode_pin(PinFunc::Output, PinPull::Default); // GPIO17 - output
    pins[18] = encode_pin(PinFunc::Output, PinPull::Default); // GPIO18 - output
    pins[22] = encode_pin(PinFunc::Input, PinPull::Down); // GPIO22 - input, pull-down
    pins[23] = encode_pin(PinFunc::Input, PinPull::Down); // GPIO23 - input, pull-down
    pins[24] = encode_pin(PinFunc::Output, PinPull::Default); // GPIO24 - output
    pins[25] = encode_pin(PinFunc::Output, PinPull::Default); // GPIO25 - output
    let gpio_atom = GpioMapAtom {
        flags: 0x00,
        power: 0x00,
        pins,
    };

    // Custom atom 1: Configuration string
    let config_str = "MODE=SENSORS,INTERVAL=250,UNITS=METRIC".to_string();
    custom_atoms.push((0x81, config_str.into_bytes())); // Using tuple format (type, data)

    // Custom atom 2: Sensor calibration data (example of binary data)
    let mut sensor_cal = Vec::new();
    // Temperature offset and gain
    sensor_cal.extend_from_slice(&((-2.5f32).to_be_bytes()));
    sensor_cal.extend_from_slice(&(1.03f32.to_be_bytes()));
    // Humidity offset and gain
    sensor_cal.extend_from_slice(&(1.2f32.to_be_bytes()));
    sensor_cal.extend_from_slice(&(0.98f32.to_be_bytes()));
    // Pressure offset and gain
    sensor_cal.extend_from_slice(&(15.0f32.to_be_bytes()));
    sensor_cal.extend_from_slice(&(1.0f32.to_be_bytes()));
    custom_atoms.push((0x82, sensor_cal)); // Using tuple format (type, data)

    // Custom atom 3: Hardware version info as string
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
    #[cfg(feature = "alloc")]
    let mut eeprom = Eeprom {
        header: EepromHeader::new(),
        vendor_info: vendor_atom,
        gpio_map_bank0: gpio_atom,
        dt_blob: None,
        gpio_map_bank1: None,
        custom_atoms,
    };

    #[cfg(not(feature = "alloc"))]
    // Для no_std режима создаем статические данные
    let mut eeprom = {
        // Для no_std нам нужны статические данные, это просто заглушка
        static CUSTOM_ATOMS: [(u8, &[u8]); 0] = [];
        Eeprom {
            header: EepromHeader::new(),
            vendor_info: vendor_atom,
            gpio_map_bank0: gpio_atom,
            dt_blob: None,
            gpio_map_bank1: None,
            custom_atoms: &CUSTOM_ATOMS,
        }
    };

    // Update header with correct counts and length
    eeprom.update_header();

    // Serialize a complete, spec-compliant HAT image (per-atom CRC-16 embedded)
    #[cfg(feature = "alloc")]
    let serialized = eeprom.serialize();

    #[cfg(not(feature = "alloc"))]
    // Создаем буфер и вектор для копирования данных
    let serialized = {
        let mut buffer = [0u8; 1024]; // Буфер достаточного размера
        let size = eeprom
            .serialize_to_slice(&mut buffer)
            .expect("Failed to serialize EEPROM");
        // Копируем данные в новый вектор
        buffer[..size].to_vec()
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
    if Eeprom::verify(&serialized) {
        println!("✅ CRC-16 verification passed");
    } else {
        println!("❌ CRC-16 verification failed");
    }

    println!("💡 This demonstrates how to embed custom application-specific data in HAT EEPROM");
}
