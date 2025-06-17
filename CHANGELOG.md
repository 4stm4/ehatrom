# Changelog

All notable changes to this project will be documented in this file.

## [0.3.1] — 2025-06-17
- **IMPROVEMENT**: Выделение библиотеки ehatrom в отдельный репозиторий
- **FIXED**: Обновлены ссылки на репозиторий в документации и метаданных
- **IMPROVEMENT**: Исправлены бейджи в README.md для нового репозитория

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
