// Advanced EEPROM creation example with Device Tree blob
use ehatrom::*;

fn main() {
    println!("🚀 Creating advanced EEPROM with Device Tree support...");

    // Create a vendor info atom with detailed information
    let vendor_atom = VendorInfoAtom::new(
        0x414C, // vendor_id (example: "AL" for AleksejZaharčenko)
        0x2024, // product_id (year)
        1,      // product_ver
        "4STM4 Ocultum",
        "Advanced HAT Demo",
        [
            // UUID for this specific HAT
            0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54,
            0x32, 0x10,
        ],
    );

    // Create GPIO map for bank 0 with specific pin configurations
    let mut gpio_pins = [0u8; 28];

    // Configure some pins as outputs (value 1)
    gpio_pins[18] = 1; // GPIO 18 as output
    gpio_pins[19] = 1; // GPIO 19 as output
    gpio_pins[20] = 1; // GPIO 20 as output
    gpio_pins[21] = 1; // GPIO 21 as output

    // Other pins remain as inputs (value 0)

    let gpio_atom = GpioMapAtom {
        flags: 0x0001, // Set some flags
        pins: gpio_pins,
    };

    // Create a simple Device Tree blob (minimal example)
    // In real use, this would be a proper compiled device tree
    let dt_blob_data = b"# Simple Device Tree overlay for demo HAT
/dts-v1/;
/plugin/;

/ {
    compatible = \"brcm,bcm2835\";
    
    fragment@0 {
        target = <&gpio>;
        __overlay__ {
            demo_pins: demo_pins {
                brcm,pins = <18 19 20 21>;
                brcm,function = <1>; /* GPIO_OUT */
            };
        };
    };
    
    fragment@1 {
        target-path = \"/\";
        __overlay__ {
            demo_hat {
                compatible = \"4stm4,demo-hat\";
                pinctrl-names = \"default\";
                pinctrl-0 = <&demo_pins>;
                status = \"okay\";
            };
        };
    };
};"
    .to_vec();

    // Create EEPROM structure with all components
    let mut eeprom = Eeprom {
        header: EepromHeader::new(),
        vendor_info: vendor_atom,
        gpio_map_bank0: gpio_atom,
        dt_blob: Some(dt_blob_data), // dt_blob is Option<Vec<u8>>
        gpio_map_bank1: None,        // Not used in this example
        custom_atoms: Vec::new(),
    };

    // Update header with correct counts and length
    eeprom.update_header();

    // Serialize with CRC
    let serialized = eeprom.serialize_with_crc();
    let filename = "tests/data/advanced.bin";

    // Create output directory if it doesn't exist
    if std::fs::metadata("tests/data").is_err() {
        std::fs::create_dir_all("tests/data").expect("Failed to create tests/data directory");
    }

    std::fs::write(filename, &serialized).expect("Failed to write advanced EEPROM file");

    println!("✅ Created {} ({} bytes)", filename, serialized.len());
    println!("📊 EEPROM structure:");
    println!("   • Header: 12 bytes");
    println!(
        "   • Vendor Info: {} bytes",
        std::mem::size_of::<VendorInfoAtom>() + "4STM4 Ocultum".len() + "Advanced HAT Demo".len()
    );
    println!(
        "   • GPIO Map Bank 0: {} bytes",
        std::mem::size_of::<GpioMapAtom>()
    );
    println!(
        "   • Device Tree Blob: {} bytes",
        serialized.len()
            - 12
            - std::mem::size_of::<VendorInfoAtom>()
            - "4STM4 Ocultum".len()
            - "Advanced HAT Demo".len()
            - std::mem::size_of::<GpioMapAtom>()
            - 4
    );
    println!("   • CRC32: 4 bytes");

    // Verify the created file
    if Eeprom::verify_crc(&serialized) {
        println!("✅ CRC32 verification passed");
    } else {
        println!("❌ CRC32 verification failed");
    }

    println!("🎯 Use './target/release/ehatrom show {filename}' to analyze the created EEPROM");
}
