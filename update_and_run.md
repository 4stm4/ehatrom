/*
  4STM4
  ehatrom
*/
# update_and_run.sh

This script automates repository updates, building, and working with real EEPROM on Raspberry Pi HAT.

## Usage

1. Make the script executable:
   
   chmod +x update_and_run.sh

2. Run the script on Raspberry Pi with connected HAT:
   
   ./update_and_run.sh

## What the script does

- **Git Pull**: Fetches latest changes from repository
- **Build**: Executes `cargo build --release --features=linux` with I2C support
- **HAT Detection**: Scans I2C bus to find EEPROM (address 0x50)
- **Test File Creation**: Runs examples to create EEPROM files
- **Testing**: Executes all tests for code quality verification
- **Real EEPROM Operations**: Safe read/write operations with backup

## Real EEPROM Operations

The script performs the following sequence of operations with HAT EEPROM:

1. **Backup**: Reads and saves existing EEPROM
2. **Content Analysis**: Shows current EEPROM structure
3. **Test Write**: Writes test EEPROM file
4. **Verification**: Reads and verifies written data
5. **Restoration**: Restores original EEPROM

## Requirements

### Hardware:
- Raspberry Pi with GPIO connector
- HAT with EEPROM (usually at I2C address 0x50)

### Software:
- Enabled I2C interface (`sudo raspi-config` -> Interface Options -> I2C)
- Installed `i2c-tools` (installed automatically)
- Superuser privileges for I2C access

## Safety

⚠️ **IMPORTANT**: The script always creates a backup before writing!

- Original EEPROM is saved with timestamp
- In case of restoration error, backup remains
- Test write is performed only if backup was successfully created

## Created Files

- `eeprom_backup_YYYYMMDD_HHMMSS.bin` - backup of original EEPROM
- `tests/data/simple.bin` - simple test EEPROM file
- `tests/data/test.bin` - full test EEPROM file with vendor info
- `tests/data/advanced.bin` - advanced EEPROM with Device Tree blob
- `tests/data/custom_atoms.bin` - EEPROM with custom atoms

## Troubleshooting

If the script cannot find EEPROM:

1. **Check I2C**: `sudo i2cdetect -y 1`
2. **Enable I2C**: `sudo raspi-config` -> Interface Options -> I2C
3. **Check modules**: `lsmod | grep i2c`
4. **Check HAT connection**: physical connection to GPIO

## Available Commands

### Working with real EEPROM:
```bash
# Read EEPROM from HAT
sudo ./target/release/ehatrom read /dev/i2c-1 0x50 backup.bin

# Show EEPROM file contents
./target/release/ehatrom show backup.bin

# Write EEPROM to HAT (CAUTION!)
sudo ./target/release/ehatrom write /dev/i2c-1 0x50 new_eeprom.bin

# Auto-detect HAT EEPROM (NEW!)
sudo ./target/release/ehatrom detect                    # Standard I2C bus (/dev/i2c-0)
sudo ./target/release/ehatrom detect /dev/i2c-1         # Specific I2C bus
sudo ./target/release/ehatrom detect --all              # Scan ALL I2C devices automatically
```

### Creating custom EEPROM files:
```bash
# Create simple EEPROM file
cargo run --example create_simple

# Create full test EEPROM file
cargo run --example create_test

# Create advanced EEPROM with Device Tree
cargo run --example create_advanced

# Create EEPROM with custom atoms
cargo run --example create_custom_atoms
```

### I2C Diagnostics:
```bash
# Scan I2C bus 1
sudo i2cdetect -y 1

# Check I2C modules
lsmod | grep i2c

# Check I2C devices
ls -la /dev/i2c*
```

## Demo Script Features

The enhanced `update_and_run.sh` script now includes comprehensive demonstrations:

### 🚀 **EEPROM Creation Examples**:
- **Simple EEPROM** (`create_simple.rs`): Minimal structure with basic vendor info
- **Full Test EEPROM** (`create_test.rs`): Complete HAT EEPROM with all standard atoms
- **Advanced EEPROM** (`create_advanced.rs`): Includes Device Tree blob for hardware configuration
- **Custom Atoms EEPROM** (`create_custom_atoms.rs`): Demonstrates user-defined data storage

### 📊 **Analysis and Verification**:
- Automatic analysis of all created EEPROM files
- CRC32 verification for data integrity
- Detailed structure breakdown showing atom types and sizes
- Hexadecimal data display for binary content

### 🔧 **Hardware Integration**:
- I2C bus scanning and device detection
- Safe backup and restore operations
- Real-time EEPROM read/write with verification
- Error handling and recovery procedures

## Notes

- Script works only on Linux (Raspberry Pi OS)
- Root privileges required for I2C device access
- HAT EEPROM typically located at I2C address 0x50
- EEPROM size usually 256 bytes (sufficient for standard HAT)
- Always backup before writing to prevent data loss

## Library Capabilities

The `ehatrom` library provides:

- **Zero external dependencies** (bare-metal compatible)
- **Custom CRC32 implementation** with compile-time table generation
- **Full HAT specification support** including all standard atom types
- **Extensible architecture** for custom atom types
- **Cross-platform development** (build on any OS, deploy on Linux)
- **Comprehensive testing** (17 test cases covering all functionality)
- **Advanced I2C device discovery** (automatic scanning of all I2C buses)
- **Smart HAT detection** (R-Pi signature verification and comprehensive error reporting)
