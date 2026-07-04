# Changelog

All notable changes to this project will be documented in this file.



## [0.4.0] — 2026-07-03
**BREAKING**: The on-disk format is now byte-compatible with the official Raspberry Pi HAT ID EEPROM format (reference `eepmake`/`eepdump`, `raspberrypi/utils/eeptools`). Images produced by earlier versions used a self-consistent but non-standard layout and will not be accepted by a Raspberry Pi bootloader or `eepdump`; regenerate them with this release.

- **BREAKING**: `AtomHeader` is now `type:u16`, `count:u16`, `dlen:u32` (little-endian), matching the spec 8-byte atom header. `dlen` now counts the atom data **plus** the trailing 2-byte CRC (`dlen = data + 2`).
- **BREAKING**: Each atom now carries its own trailing **CRC-16** (reflected, polynomial `0x8005`, a.k.a. CRC-16/ARC) computed over the atom header and data — exactly as `eepmake` does. The previous single whole-image CRC-32 is gone.
- **BREAKING**: `VendorInfoAtom` follows `struct vendor_info_d`: `uuid`/`pid`/`pver` plus variable-length vendor/product strings with `vslen`/`pslen`. The non-spec `vendor_id` field was removed; `VendorInfoAtom::new` no longer takes it.
- **BREAKING**: `GpioMapAtom` is now `flags:u8`, `power:u8`, `pins[28]` (was `flags:u16`, `pins[28]`).
- **BREAKING**: Atom type ids corrected to the spec values — vendor `0x0001`, GPIO bank0 `0x0002`, DT blob `0x0003`, custom `0x0004`, GPIO bank1 `0x0005` (was `0x04`, which collided with custom data). Custom atoms are always emitted with type `0x0004`.
- **BREAKING**: Pin `func_sel` encoding corrected — `0x00` = input, `0x01` = output (was `0x01`/`0x02`).
- **BREAKING**: `custom_atoms` is now `Vec<Vec<u8>>` (was `Vec<(u8, Vec<u8>)>`) and `add_custom_atom` drops the tag argument. The HAT format has no per-custom-atom sub-type — every custom atom is type `0x0004` — so the old tag was never serialized; custom atoms now round-trip losslessly.
- **BREAKING**: API changes — `serialize_with_crc`/`serialize_with_crc_to_slice`/`serialize_to_buffer` replaced by `serialize()` (alloc), `serialize_to_slice()` (no_std) and `serialize_into()` (both), which emit a complete valid image. `verify_crc` replaced by `verify()`, which checks every per-atom CRC-16.
- **FIXED**: Serialization no longer casts `#[repr(C, packed)]` structs through raw pointers; all fields are written/read as explicit little-endian, so output is identical on big-endian hosts.
- **ADDED**: `tests/hat_golden.rs` — a byte-exact golden image plus a CRC-16 reference check value (`crc16(b"123456789") == 0xBB3D`) that pins compatibility with the reference tools.
- **ADDED**: typed GPIO pin encoding (`PinFunc`, `PinPull`, `encode_pin`/`decode_pin`, `UNUSED_PIN`) matching `eepmake`'s `setgpio` bit layout, including the "board uses this pin" flag (bit 7).
- **ADDED**: power-supply atom (`0x0006`) support via `Eeprom::power_supply` / `add_power_supply(current_ma)`, serialized and parsed as a little-endian `u32` (milliamps).
- **ADDED**: `parse_settings()` — build an `Eeprom` from an `eepmake`-style `settings.txt`, and CLI subcommands `make`, `dump`, and `verify`.
- **ADDED**: `atoms()` — a zero-copy, `no_std` iterator over an image's atoms yielding `AtomRef`s (type, count, borrowed data, per-atom CRC validity).
- **ADDED**: an acceptance harness (`tests/acceptance/eepmake_compat.sh`, `make acceptance`) that checks ehatrom's output byte-for-byte against the reference `eepmake` and confirms `eepdump` parses it. Not part of CI (needs the external tools); run locally.
- **ADDED**: `Eeprom::serialize_to_writer()` (std) streams the image to any `std::io::Write` without buffering the whole thing, plus an incremental `utils::crc16::Crc16` (new/update/finalize).
- **ADDED**: `Eeprom::validate()` returning a `ValidationError` that names the offending atom (bad signature, truncation, `dlen`, or CRC-16 mismatch); `verify()` is now a boolean wrapper over it.

## [0.3.3] — 2025-11-22
- **IMPROVEMENT**: All main structures and fields are now public for easier integration in external projects.
- **IMPROVEMENT**: Documentation and examples updated for the new series release.
- **FIXED**: Final cleanup and refactoring before the new series release.

## [0.3.2] — 2025-06-17
- **IMPROVEMENT**: Added support for reading and writing large EEPROMs (up to several megabytes). Buffer size is now configurable via the `EHATROM_BUFFER_SIZE` environment variable for both CLI and detection commands. Default is 32KB, but you can set any value (e.g., `EHATROM_BUFFER_SIZE=1048576` for 1MB).
- **FIXED**: Improved I2C reading reliability by implementing page-based read (32 bytes per read operation) for better compatibility with real EEPROM chips that don't support reading large blocks at once.

## [0.3.1] — 2025-06-17
- **IMPROVEMENT**: Extracted ehatrom library into a separate repository
- **FIXED**: Updated repository links in documentation and metadata
- **IMPROVEMENT**: Fixed badges in README.md for the new repository

## [0.3.0] — 2025-06-17
- **BREAKING**: **Bare-Metal Support** - Library now supports `#![no_std]` environments
  - Conditional compilation with feature flags: `alloc`, `std`, `linux`
  - Alternative APIs for no-allocation environments (`serialize_to_slice`, `serialize_to_buffer`)
  - Static data support for embedded systems (`from_bytes_no_alloc`, `set_custom_atoms`)
  - Custom `EhatromError` type for better no_std error handling
- **BREAKING**: I2C functions now return `EhatromError` instead of `Box<dyn std::error::Error>`
- **BREAKING**: **Simplified CLI Interface** - Commands now automatically use HAT EEPROM address (0x50)
  - `read [i2c-dev] <output.bin>` - No longer requires manual address specification
  - `write [i2c-dev] <input.bin>` - No longer requires manual address specification
  - Default I2C device: `/dev/i2c-0` (HAT standard)
  - **Old**: `sudo ehatrom read /dev/i2c-0 0x50 output.bin`
  - **New**: `sudo ehatrom read output.bin` (uses defaults) or `sudo ehatrom read /dev/i2c-1 output.bin`
- **NEW**: Bare-metal example (`examples/bare_metal_example.rs`)
- **IMPROVED**: Cross-platform compatibility with proper `#[cfg]` attributes
- **IMPROVED**: Enhanced CLI with Linux feature detection and better help messages
- **Feature Flags**: 
  - `default = ["alloc"]` - Standard usage with heap allocation
  - `alloc` - Enable Vec/String types (requires alloc crate)
  - `std` - Enable full std library features (implies alloc)
  - `linux` - Enable I2C device support (requires i2cdev, std)

## [0.2.0] — 2025-06-13
- **BREAKING**: Removed all serde dependencies (serde, serde_json, serde_yaml, serde-xml-rs)
- **BREAKING**: Removed JSON/YAML/XML CLI commands, kept only `read`, `write`, `show`, `detect`
- **BREAKING**: Changed test data file extensions from `.eep` to `.bin` for clarity
- **BREAKING**: I2C functions now return `EhatromError` instead of `Box<dyn std::error::Error>`
- **NEW**: **Bare-Metal Support** - Library now supports `#![no_std]` environments
  - Conditional compilation with feature flags: `alloc`, `std`, `linux`
  - Alternative APIs for no-allocation environments (`serialize_to_slice`, `serialize_to_buffer`)
  - Static data support for embedded systems (`from_bytes_no_alloc`, `set_custom_atoms`)
  - Custom `EhatromError` type for better no_std error handling
- **NEW**: `detect` command for auto-detecting HAT EEPROM on I2C bus
- **NEW**: `detect --all` command to automatically scan all available I2C devices
- **NEW**: `find_i2c_devices()` function to discover all I2C devices in /dev
- **NEW**: Custom CRC32 implementation - no external dependencies for CRC32
- **NEW**: Reorganized project structure - all tests in `tests/`, examples in `examples/`
- **NEW**: Enhanced update_and_run.sh script with examples demonstration
- **NEW**: Bare-metal example (`examples/bare_metal_example.rs`)
- **IMPROVED**: Display implementations for all main types (Eeprom, EepromHeader, AtomHeader, etc.)
- **IMPROVED**: Better error handling and CLI user experience with detailed help
- **IMPROVED**: Bare-metal compatibility - minimal dependencies, works from microcontrollers to servers
- **IMPROVED**: Comprehensive documentation with usage examples
- **IMPROVED**: ARM/ARM64 support with performance optimizations for low-power devices
- **IMPROVED**: Enhanced CLI with comprehensive help and usage examples
- **IMPROVED**: Cross-platform compatibility with proper `#[cfg]` attributes
- **FIXED**: All clippy warnings and formatting issues
- **FIXED**: I2C function import issues with proper feature gating
- **FIXED**: HAT specification compliance - default I2C device is now /dev/i2c-0
- **FIXED**: ARM performance test thresholds (dynamic: 10 MB/s ARM, 50 MB/s others)
- Documentation: Updated README, CLI usage examples, Russian comments → English
- Tests: 17 comprehensive tests including performance tests for CRC32
- Zero external dependencies by default, only `i2cdev` with "linux" feature
- CI: Docker and local CI scripts for quality assurance
- **Feature Flags**: 
  - `default = ["alloc"]` - Standard usage with heap allocation
  - `alloc` - Enable Vec/String types (requires alloc crate)
  - `std` - Enable full std library features (implies alloc)
  - `linux` - Enable I2C device support (requires i2cdev, std)

## [0.1.0] — 2025-06-08
- Initial release: Raspberry Pi HAT EEPROM library
- Full EEPROM (de)serialization, CRC32, I2C (Linux), custom atoms
- 100% test coverage, CLI example, EN/RU docs, CI, Docker, ASCII logo
