[![Ehatrom CI](https://github.com/4stm4/ehatrom/actions/workflows/ehatrom-rust.yml/badge.svg?branch=main)](https://github.com/4stm4/ehatrom/actions/workflows/ehatrom-rust.yml)
[![Crates.io](https://img.shields.io/crates/v/ehatrom.svg)](https://crates.io/crates/ehatrom)
[![Docs.rs](https://docs.rs/ehatrom/badge.svg)](https://docs.rs/ehatrom)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

# ehatrom — library for working with Raspberry Pi HAT EEPROM

`ehatrom` is a Rust library for reading, writing, and generating EEPROM content for Raspberry Pi HAT (Hardware Attached on Top) via I2C. It supports correct serialization/deserialization of the structure, working with atoms (VendorInfo, GPIO Map, DTBlob, custom), reading/writing with 2-byte offset and page write, and convenient content output.

The generated images are **byte-compatible with the official Raspberry Pi HAT ID EEPROM format** as produced/parsed by the reference `eepmake`/`eepdump` tools ([`raspberrypi/utils`, `eeptools`](https://github.com/raspberrypi/utils/tree/master/eeptools)): 12-byte header, 8-byte atom headers (`type:u16`, `count:u16`, `dlen:u32`, all little-endian, where `dlen = data + 2`), and a per-atom CRC-16 (reflected, polynomial `0x8005`) over each atom header and its data. See [tests/hat_golden.rs](tests/hat_golden.rs) for a byte-exact golden image.

## Features
- Read and write Raspberry Pi HAT EEPROM via I2C (with page write and 2-byte offset support)
- Serialization and parsing of EEPROM structure in the official Raspberry Pi HAT format (spec atom types, per-atom CRC-16)
- Endianness-independent output (all fields written as little-endian regardless of host)
- Convenient content output, including string fields
- CLI example for reading/writing/dumping EEPROM
- Support for manufacturer custom atoms (spec type `0x0004`)
- Large EEPROM support with configurable buffer size via `EHATROM_BUFFER_SIZE` environment variable
- Page-based reading implementation (32 bytes per read) for better compatibility with real EEPROM chips

## Structures
- `EepromHeader` — EEPROM header
- `AtomHeader` — atom header
- `VendorInfoAtom` — vendor and product info
- `GpioMapAtom` — GPIO map (28 pins per bank)
- `DtBlobAtom` — device tree blob
- `Eeprom` — full EEPROM structure

### Why 28 pins in GpioMapAtom?
28 pins correspond to GPIO0–GPIO27 of the standard 40-pin Raspberry Pi header. This is exactly the number of user GPIOs available on regular models. For extended boards (Compute Module), a second atom (GPIO map bank1, spec type `0x0005`) can be added; on the wire a bank1 atom carries 18 pins (GPIO28–GPIO45).

## Usage Example

```rust
use ehatrom::{Eeprom, EepromHeader, VendorInfoAtom, GpioMapAtom};

// Create VendorInfoAtom. The HAT vendor atom has no separate "vendor_id" field;
// the vendor is identified by its string plus the 16-byte UUID.
let vendor_info = VendorInfoAtom::new(
    0x5678,     // product_id (pid)
    1,          // product_ver (pver)
    "MyVendor", // vendor string (stored in a 16-byte buffer)
    "MyHAT",    // product string (stored in a 16-byte buffer)
    [0u8; 16],  // uuid / serial
);

// Fill GPIO map. Per the spec func_sel encoding: 0x00 = input, 0x01 = output.
let mut pins = [0u8; 28];
pins[4] = 0x00;   // GPIO4 — input
pins[17] = 0x01;  // GPIO17 — output
let gpio_map = GpioMapAtom { flags: 0, power: 0, pins };

let mut eeprom = Eeprom {
    header: EepromHeader::new(),
    vendor_info,
    gpio_map_bank0: gpio_map,
    dt_blob: None,
    gpio_map_bank1: None,
    custom_atoms: Vec::new(),
};
eeprom.update_header();

// Serialize a complete, spec-compliant HAT image. Each atom already carries its
// own trailing CRC-16 — there is no separate whole-image checksum step.
let bytes = eeprom.serialize();

// Write to EEPROM via I2C (validates every per-atom CRC-16 first)
// ehatrom::write_to_eeprom_i2c(&bytes, "/dev/i2c-1", 0x50)?;

// Read from EEPROM and verify
// let mut buf = vec![0u8; 256];
// ehatrom::read_from_eeprom_i2c(&mut buf, "/dev/i2c-1", 0x50, 0)?;
// if Eeprom::verify(&buf) {
//     let eeprom = Eeprom::from_bytes(&buf)?;
//     println!("{:?}", eeprom);
// } else {
//     println!("CRC check failed!");
// }

// Add manufacturer custom atoms. All custom atoms are emitted with the spec
// type 0x0004; the first tuple element is an informational tag only.
eeprom.add_custom_atom(0x00, b"serial:1234567890".to_vec());
eeprom.add_custom_atom(0x00, b"api_url:https://api.example.com/v1".to_vec());
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
Each byte of the `pins` array encodes one GPIO, matching `eepmake`'s `setgpio`:

| bits  | meaning                                              |
|-------|------------------------------------------------------|
| `2:0` | `func_sel` — input=0, output=1, ALT0..ALT5 = 4,5,6,7,3,2 |
| `4:3` | reserved (0)                                         |
| `6:5` | pull — 0=default, 1=up, 2=down, 3=none               |
| `7`   | "board uses this pin" flag                           |

Note that `0x00` means **the pin is not used by the board**, not "input": a used input pin with the default pull is `0x80`. Prefer the typed helpers over raw bytes:

```rust
use ehatrom::{encode_pin, decode_pin, PinFunc, PinPull, UNUSED_PIN};

let mut pins = [UNUSED_PIN; 28];
pins[17] = encode_pin(PinFunc::Output, PinPull::Default); // 0x81
pins[4]  = encode_pin(PinFunc::Input,  PinPull::Up);      // 0xA0

let cfg = decode_pin(pins[17]);
assert_eq!(cfg.func, PinFunc::Output);
```

## Platform Support

- The core library (EEPROM structures, serialization, CRC, etc.) is **cross-platform** and works on any OS (Linux, macOS, Windows, etc.).
- **I2C EEPROM read/write functions** (`write_to_eeprom_i2c`, `read_from_eeprom_i2c`) are available **only on Linux** (using the [i2cdev](https://crates.io/crates/i2cdev) crate).
- The I2C reading implementation uses **page-based reading** (32 bytes per read operation) for better compatibility with real EEPROM chips that don't support reading large blocks at once.
- Buffer size for I2C operations is configurable via the `EHATROM_BUFFER_SIZE` environment variable (default is 32KB, but can be set to any value up to several megabytes).
- On other platforms, you can use all parsing/serialization features, but direct I2C access is not available.

## Dependencies

- CRC-16 (HAT format) and CRC-32 are implemented in-crate (`no_std`, no external crate)
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

# Read large EEPROM with custom buffer size (default is 32KB)
EHATROM_BUFFER_SIZE=1048576 sudo ehatrom read large_eeprom.bin  # 1MB buffer

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

# Use a larger buffer for detecting EEPROMs larger than 32KB
EHATROM_BUFFER_SIZE=1048576 sudo ehatrom detect  # 1MB buffer
```

- All errors and usage info are printed to stderr.
- Requires root for I2C access on Linux.

## Links
- [Official HAT EEPROM specification](https://github.com/raspberrypi/hats/blob/master/eeprom-format.md)
- [Reference `eeptools` (`eepmake`/`eepdump`)](https://github.com/raspberrypi/utils/tree/master/eeptools) — the format `ehatrom` targets byte-for-byte

## License
MIT
