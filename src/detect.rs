//  _  _       _             _  _
// | || |  ___| |_ _ __ ___ | || |
// | || |_/ __| __| '_ ` _ \| || |_
// |__   _\__ | |_| | | | | |__   _|
//   |_| |___/\__|_|_|_| |_|  |_|
//  ___   ___ _   _| | |_ _   _ _ __ ___
// / _ \ / __| | | | | __| | | | '_ ` _ \
//| (_) | (__| |_| | | |_| |_| | | | | | |
// \___/ \___|\__,_|_|\__|\__,_|_| |_| |_|
//! # ehatrom — EEPROM HAT library for Raspberry Pi HATs
//! - [Documentation (docs.rs)](https://docs.rs/ehatrom)
//! - [GitHub](https://github.com/4stm4/ehatrom)

// Import necessary types based on features
#[cfg(feature = "alloc")]
use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};

#[cfg(feature = "std")]
use std::{print, println};

#[cfg(all(feature = "linux", any(target_os = "linux", target_os = "android")))]
fn print_header_diagnostics(buf: &[u8], buffer_len: usize) {
    if buf.len() < 12 {
        eprintln!(
            "⚠️ Warning: Not enough data to read EEPROM header ({} bytes available)",
            buf.len()
        );
        return;
    }

    let version = buf[4];
    let reserved = buf[5];
    let numatoms = u16::from_le_bytes([buf[6], buf[7]]);
    let eeplen = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);

    println!("EEPROM header analysis:");
    println!("  Version: {}", version);
    println!("  Reserved: {}", reserved);
    println!("  Number of atoms: {}", numatoms);
    println!("  EEPROM length: {} bytes", eeplen);

    if numatoms == 0 {
        eprintln!("⚠️ Warning: Header indicates 0 atoms, which is invalid");
    }
    if eeplen < 12 {
        eprintln!("❌ Invalid EEPROM length: {} (should be >= 12)", eeplen);
    } else if eeplen as usize > buffer_len {
        eprintln!(
            "⚠️ Warning: Header indicates EEPROM length ({} bytes) is larger than read buffer ({} bytes)",
            eeplen, buffer_len
        );
    }
}

#[cfg(all(feature = "linux", any(target_os = "linux", target_os = "android")))]
fn read_eeprom_with_dynamic_buffer(
    dev_path: &str,
    addr: u16,
    initial_len: usize,
) -> Result<Vec<u8>, crate::EhatromError> {
    use crate::read_from_eeprom_i2c;

    const MIN_HEADER_LEN: usize = 12;
    let mut header_buf = vec![0u8; MIN_HEADER_LEN];
    read_from_eeprom_i2c(&mut header_buf, dev_path, addr, 0)?;

    let eeplen = if &header_buf[0..4] == b"R-Pi" {
        Some(
            u32::from_le_bytes([header_buf[8], header_buf[9], header_buf[10], header_buf[11]])
                as usize,
        )
    } else {
        None
    };

    let desired_len = initial_len
        .max(MIN_HEADER_LEN)
        .max(eeplen.unwrap_or(MIN_HEADER_LEN));

    if desired_len <= header_buf.len() {
        return Ok(header_buf);
    }

    let mut buf = vec![0u8; desired_len];
    read_from_eeprom_i2c(&mut buf, dev_path, addr, 0)?;
    Ok(buf)
}

#[cfg(all(feature = "linux", any(target_os = "linux", target_os = "android")))]
/// Scans the provided I2C device, printing parsed EEPROM details if found.
///
/// Returns [`EhatromError`](crate::EhatromError) when I2C access or parsing fails.
pub fn detect_and_show_eeprom_info(
    dev_path: &str,
    read_len: usize,
) -> Result<(), crate::EhatromError> {
    use crate::Eeprom;

    const HAT_EEPROM_ADDR: u16 = 0x50;

    println!("Scanning I2C bus {} for HAT EEPROM...", dev_path);
    println!("Using address: 0x{:02X}", HAT_EEPROM_ADDR);
    print!("Trying 0x{:02X}... ", HAT_EEPROM_ADDR);

    let buf = match read_eeprom_with_dynamic_buffer(dev_path, HAT_EEPROM_ADDR, read_len) {
        Ok(buf) => buf,
        Err(e) => {
            eprintln!("read error: {}", e);
            println!("No valid Raspberry Pi HAT EEPROM found on bus {}", dev_path);
            return Ok(());
        }
    };

    if buf.len() >= 4 && &buf[0..4] == b"R-Pi" {
        println!("Found HAT EEPROM!");
        println!("First 16 bytes: {:02X?}", &buf[0..16.min(buf.len())]);

        print_header_diagnostics(&buf, buf.len());

        match Eeprom::from_bytes(&buf) {
            Ok(eeprom) => {
                println!("EEPROM found at 0x{:02X} on {}", HAT_EEPROM_ADDR, dev_path);
                println!("{eeprom}");
                return Ok(());
            }
            Err(e) => {
                eprintln!(
                    "EEPROM found at 0x{:02X} but failed to parse: {}",
                    HAT_EEPROM_ADDR, e
                );
                if buf.len() >= 64 {
                    println!("Raw data (first 64 bytes): {:02X?}", &buf[0..64]);
                }
            }
        }
    } else {
        println!(
            "no HAT signature (first 4 bytes: {:02X?})",
            &buf[0..4.min(buf.len())]
        );
    }

    println!("No valid Raspberry Pi HAT EEPROM found on bus {}", dev_path);
    Ok(())
}

#[cfg(all(feature = "linux", any(target_os = "linux", target_os = "android")))]
/// Returns a sorted list of `/dev/i2c-*` device paths available on the host.
pub fn find_i2c_devices() -> Vec<String> {
    #[cfg(feature = "std")]
    use std::fs;

    let mut devices = Vec::new();

    // Scan /dev for i2c-* devices
    if let Ok(entries) = fs::read_dir("/dev") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("i2c-") && name.len() > 4 {
                    // Check if it's a valid i2c device by trying to parse the number
                    if let Ok(_) = name[4..].parse::<u32>() {
                        devices.push(path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    // Sort devices by number for consistent output
    devices.sort_by(|a, b| {
        let num_a = a
            .split('-')
            .last()
            .unwrap_or("0")
            .parse::<u32>()
            .unwrap_or(0);
        let num_b = b
            .split('-')
            .last()
            .unwrap_or("0")
            .parse::<u32>()
            .unwrap_or(0);
        num_a.cmp(&num_b)
    });

    devices
}

#[cfg(all(feature = "linux", any(target_os = "linux", target_os = "android")))]
/// Scans all detected I2C buses and attempts to find HAT EEPROMs.
pub fn detect_all_i2c_devices() -> Result<(), crate::EhatromError> {
    let devices = find_i2c_devices();

    if devices.is_empty() {
        println!("No I2C devices found in /dev");
        println!("Make sure I2C is enabled and you have proper permissions.");
        return Ok(());
    }

    println!("Found {} I2C device(s): {:?}", devices.len(), devices);
    println!();

    // Support reading large EEPROMs - default buffer is 32 KB for detection
    // Custom size can be set via the EHATROM_BUFFER_SIZE environment variable
    let read_len = match std::env::var("EHATROM_BUFFER_SIZE") {
        Ok(size_str) => match size_str.parse::<usize>() {
            Ok(size) => {
                if size < 1024 {
                    eprintln!("Warning: Buffer size too small, using minimum 1KB");
                    1024
                } else {
                    println!("Using custom buffer size: {} bytes", size);
                    size
                }
            }
            Err(_) => {
                eprintln!("Warning: Failed to parse EHATROM_BUFFER_SIZE, using 32KB");
                32 * 1024
            }
        },
        Err(_) => 32 * 1024, // 32 KB by default
    };

    let mut found_any = false;

    for device in &devices {
        println!("=== Scanning {} ===", device);
        match detect_and_show_eeprom_info(device, read_len) {
            Ok(_) => {
                found_any = true;
                println!();
            }
            Err(e) => {
                println!("Error scanning {}: {}", device, e);
                println!();
            }
        }
    }

    if !found_any {
        println!("No HAT EEPROM found on any I2C device.");
        println!("This could mean:");
        println!("  • No HAT is connected");
        println!("  • HAT EEPROM is not programmed");
        println!("  • HAT uses a different I2C address");
        println!("  • Permissions issue (try running with sudo)");
    }

    Ok(())
}

#[cfg(not(all(feature = "linux", any(target_os = "linux", target_os = "android"))))]
/// Stub for platforms without Linux I2C support; always returns an empty list.
pub fn find_i2c_devices() -> Vec<String> {
    Vec::new()
}

#[cfg(not(all(feature = "linux", any(target_os = "linux", target_os = "android"))))]
/// Stub for platforms without Linux I2C support; exits with an error when used.
pub fn detect_all_i2c_devices() -> Result<(), crate::EhatromError> {
    #[cfg(feature = "std")]
    {
        eprintln!("I2C device detection is only supported on Linux with --features=linux");
        std::process::exit(1);
    }
    #[cfg(not(feature = "std"))]
    Err(crate::EhatromError::I2cError)
}

#[cfg(not(all(feature = "linux", any(target_os = "linux", target_os = "android"))))]
/// Stub for platforms without Linux I2C support; returns an error or exits.
pub fn detect_and_show_eeprom_info(
    _dev_path: &str,
    _read_len: usize,
) -> Result<(), crate::EhatromError> {
    #[cfg(feature = "std")]
    {
        eprintln!("EEPROM detection is only supported on Linux with --features=linux");
        std::process::exit(1);
    }
    #[cfg(not(feature = "std"))]
    Err(crate::EhatromError::I2cError)
}
