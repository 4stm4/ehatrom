[![Ehatrom CI](https://github.com/4stm4/ehatrom/actions/workflows/ehatrom-rust.yml/badge.svg?branch=main)](https://github.com/4stm4/ehatrom/actions/workflows/ehatrom-rust.yml)
[![Crates.io](https://img.shields.io/crates/v/ehatrom.svg)](https://crates.io/crates/ehatrom)
[![Docs.rs](https://docs.rs/ehatrom/badge.svg)](https://docs.rs/ehatrom)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

# ehatrom — library for working with Raspberry Pi HAT EEPROM

`ehatrom` is a Rust library for reading, writing, and generating EEPROM content for Raspberry Pi HAT (Hardware Attached on Top) via I2C. It supports correct serialization/deserialization of the structure, working with atoms (VendorInfo, GPIO Map, DTBlob, custom), reading/writing with 2-byte offset and page write, and convenient content output.

## Features
- Read and write Raspberry Pi HAT EEPROM via I2C (with page write and 2-byte offset support)
- Serialization and parsing of EEPROM structure according to the official Raspberry Pi HAT specification
- Convenient content output, including string fields
- CLI example for reading/writing/dumping EEPROM
- Support for custom atoms and CRC32 integrity check

## Structures
- `EepromHeader` — EEPROM header
- `AtomHeader` — atom header
- `VendorInfoAtom` — vendor and product info
- `GpioMapAtom` — GPIO map (28 pins per bank)
- `DtBlobAtom` — device tree blob
- `Eeprom` — full EEPROM structure

### Why 28 pins in GpioMapAtom?
28 pins correspond to GPIO0–GPIO27 of the standard 40-pin Raspberry Pi header. This is exactly the number of user GPIOs available on regular models. For extended boards (Compute Module), a second atom (GpioMapBank1) can be added.

## Usage Example

```rust
use ehatrom::{Eeprom, VendorInfoAtom, GpioMapAtom};

// Create VendorInfoAtom
let vendor_info = VendorInfoAtom::new(
    0x1234, // vendor_id
    0x5678, // product_id
    1,      // product_ver
    "MyVendor", // vendor (any string length)
    "MyHAT",    // product (any string length)
    [0u8; 16],   // uuid
);

// Fill GPIO map: all unused (0), GPIO4 — input (0x01), GPIO17 — output (0x02)
let mut pins = [0u8; 28];
pins[4] = 0x01;   // GPIO4 — input
pins[17] = 0x02;  // GPIO17 — output
let gpio_map = GpioMapAtom { flags: 0, pins };

let mut eeprom = Eeprom {
    header: Default::default(),
    vendor_info,
    gpio_map_bank0: gpio_map,
    dt_blob: None,
    gpio_map_bank1: None,
    custom_atoms: Vec::new(),
};
eeprom.update_header();

// Serialization to bytes
let bytes = eeprom.serialize();

// Serialization with CRC32
let bytes_with_crc = eeprom.serialize_with_crc();

// Write to EEPROM via I2C
// ehatrom::write_to_eeprom_i2c(&bytes_with_crc, "/dev/i2c-1", 0x50)?;

// Read from EEPROM with CRC check
// let mut buf = vec![0u8; 256];
// ehatrom::read_from_eeprom_i2c(&mut buf, "/dev/i2c-1", 0x50, 0)?;
// if Eeprom::verify_crc(&buf) {
//     let eeprom = Eeprom::from_bytes(&buf[..buf.len()-4])?;
//     println!("{:?}", eeprom);
// } else {
//     println!("CRC check failed!");
// }

// Add a custom atom (e.g., with settings or serial number)
let custom_data = b"serial:1234567890".to_vec();
eeprom.add_custom_atom(0x80, custom_data);

// Add a custom atom with settings (e.g., API address)
let api_url = b"api_url:https://api.example.com/v1".to_vec();
eeprom.add_custom_atom(0x80, api_url);
let api_key = b"api_key:SECRET123456".to_vec();
eeprom.add_custom_atom(0x81, api_key);
```

## Setting EEPROM Version

By default, the version is set to 1. To set a custom version (for example, 2):

```rust
let mut eeprom = Eeprom { header: Default::default(), /* ... */ };
eeprom.set_version(2); // set version to 2
```

Or, using the builder pattern:

```rust
let mut eeprom = Eeprom { header: Default::default(), /* ... */ };
// ... fill other fields ...
eeprom.set_version(3); // set version to 3
```

## pins field format
Each byte of the `pins` array defines the function of the corresponding GPIO:
- 0x00 — unused
- 0x01 — input
- 0x02 — output
- ... (see HAT EEPROM specification)

## Platform Support

- The core library (EEPROM structures, serialization, CRC, etc.) is **cross-platform** and works on any OS (Linux, macOS, Windows, etc.).
- **I2C EEPROM read/write functions** (`write_to_eeprom_i2c`, `read_from_eeprom_i2c`) are available **only on Linux** (using the [i2cdev](https://crates.io/crates/i2cdev) crate).
- On other platforms, you can use all parsing/serialization features, but direct I2C access is not available.

## Dependencies

- [crc32fast](https://crates.io/crates/crc32fast) — for CRC32 calculation
- [i2cdev](https://crates.io/crates/i2cdev) — for I2C access (Linux only)

See also: [update_and_run.md](./update_and_run.md) for usage automation.

## Command-line interface (CLI)

A full-featured CLI is available starting from version 0.3.0:

```
Usage: ehatrom <read|write|show|detect> [options]

Commands:
  read [i2c-dev] <output.bin>             Read EEPROM via I2C and save to file
  write [i2c-dev] <input.bin>             Write EEPROM from file to I2C device
  show <input.bin>                        Show parsed EEPROM info from file (debug format)
  detect [i2c-dev]                        Auto-detect HAT EEPROM on specific device (default: /dev/i2c-0)
  detect --all                            Scan all available I2C devices for HAT EEPROM
```

Examples:

```sh
# Read EEPROM to file (uses default /dev/i2c-0 and address 0x50)
sudo ehatrom read dump.bin

# Read EEPROM from specific I2C device
sudo ehatrom read /dev/i2c-1 dump.bin

# Write EEPROM from file (uses default /dev/i2c-0 and address 0x50)
sudo ehatrom write dump.bin

# Write EEPROM to specific I2C device
sudo ehatrom write /dev/i2c-1 dump.bin

# Show EEPROM info (debug format)
./ehatrom show dump.bin

# Auto-detect HAT EEPROM and show info
sudo ehatrom detect                    # Uses /dev/i2c-0 by default (HAT EEPROM standard)
sudo ehatrom detect /dev/i2c-1        # Use specific I2C bus if needed
sudo ehatrom detect --all             # Scan all I2C devices automatically
```

### Advanced Detection Features

The `detect --all` command automatically finds all available I2C devices in `/dev` (like `/dev/i2c-0`, `/dev/i2c-1`, etc.) and scans each one for HAT EEPROM. This is especially useful when you're not sure which I2C bus your HAT is connected to:

```sh
# Scan all I2C devices - shows which devices are available and which contain HAT EEPROM
sudo ehatrom detect --all
```

- All errors and usage info are printed to stderr.
- Requires root for I2C access on Linux.

## Links
- [Official HAT EEPROM specification](https://github.com/raspberrypi/hats/blob/master/eeprom-format.md)

## License
MIT
