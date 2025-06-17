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
pub fn detect_and_show_eeprom_info(
    dev_path: &str,
    possible_addrs: &[u16],
    read_len: usize,
) -> Result<(), crate::EhatromError> {
    use crate::{Eeprom, read_from_eeprom_i2c};

    println!("Scanning I2C bus {} for HAT EEPROM...", dev_path);
    println!("Checking addresses: {:02X?}", possible_addrs);

    for &addr in possible_addrs {
        print!("Trying 0x{:02X}... ", addr);
        let mut buf = vec![0u8; read_len];
        match read_from_eeprom_i2c(&mut buf, dev_path, addr, 0) {
            Ok(_) => {
                if buf.len() >= 4 && &buf[0..4] == b"R-Pi" {
                    println!("Found HAT EEPROM!");
                    // Show first 16 bytes for debugging
                    println!("First 16 bytes: {:02X?}", &buf[0..16.min(buf.len())]);

                    // Дополнительная диагностика заголовка
                    if buf.len() >= 12 {
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
                            println!("⚠️ Warning: Header indicates 0 atoms, which is invalid");
                        }
                        if eeplen as usize > buf.len() {
                            println!(
                                "⚠️ Warning: Header indicates EEPROM length ({} bytes) is larger than read buffer ({} bytes)",
                                eeplen,
                                buf.len()
                            );
                            println!(
                                "   Consider using EHATROM_BUFFER_SIZE={} to read the full EEPROM",
                                (eeplen as usize + 1024).max(buf.len() * 2)
                            ); // Предлагаем бОльший размер буфера
                        }
                    }

                    // Дополнительная диагностика заголовка
                    if buf.len() >= 12 {
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
                            println!("⚠️ Warning: Header indicates 0 atoms, which is invalid");
                        }
                        if eeplen as usize > buf.len() {
                            println!(
                                "⚠️ Warning: Header indicates EEPROM length ({} bytes) is larger than read buffer ({} bytes)",
                                eeplen,
                                buf.len()
                            );
                        }
                    }

                    match Eeprom::from_bytes(&buf) {
                        Ok(eeprom) => {
                            println!("EEPROM found at 0x{:02X} on {}", addr, dev_path);
                            println!("{eeprom}");
                            return Ok(());
                        }
                        Err(e) => {
                            println!("EEPROM found at 0x{:02X} but failed to parse: {}", addr, e);
                            // Улучшенная диагностика
                            if buf.len() >= 64 {
                                println!("Raw data (first 64 bytes): {:02X?}", &buf[0..64]);
                            }

                            // Подробная проверка структуры
                            if buf.len() >= 12 {
                                // Проверка сигнатуры
                                if &buf[0..4] != b"R-Pi" {
                                    println!(
                                        "❌ Invalid signature: expected 'R-Pi', found '{:?}'",
                                        String::from_utf8_lossy(&buf[0..4])
                                    );
                                }

                                // Проверка версии
                                let version = buf[4];
                                if version == 0 {
                                    println!("❌ Invalid version: 0 (should be > 0)");
                                }

                                // Проверка атомов
                                let numatoms = u16::from_le_bytes([buf[6], buf[7]]);
                                if numatoms == 0 {
                                    println!("❌ Invalid atom count: 0 (should be > 0)");
                                }

                                // Проверка размера
                                let eeplen = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
                                if eeplen < 12 {
                                    println!(
                                        "❌ Invalid EEPROM length: {} (should be >= 12)",
                                        eeplen
                                    );
                                }
                                if eeplen as usize > buf.len() {
                                    println!(
                                        "⚠️ EEPROM data truncated: expected {} bytes, but read only {} bytes",
                                        eeplen,
                                        buf.len()
                                    );
                                }
                            }

                            // Проверка CRC
                            if Eeprom::verify_crc(&buf) {
                                println!("✅ CRC verification passed");
                            } else {
                                println!("❌ CRC verification failed");
                            }
                        }
                    }
                } else {
                    println!(
                        "no HAT signature (first 4 bytes: {:02X?})",
                        &buf[0..4.min(buf.len())]
                    );
                }
            }
            Err(e) => {
                println!("read error: {}", e);
            }
        }
    }
    println!("No valid Raspberry Pi HAT EEPROM found on bus {}", dev_path);
    Ok(())
}

#[cfg(all(feature = "linux", any(target_os = "linux", target_os = "android")))]
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
pub fn detect_all_i2c_devices() -> Result<(), crate::EhatromError> {
    let devices = find_i2c_devices();

    if devices.is_empty() {
        println!("No I2C devices found in /dev");
        println!("Make sure I2C is enabled and you have proper permissions.");
        return Ok(());
    }

    println!("Found {} I2C device(s): {:?}", devices.len(), devices);
    println!();

    let possible_addrs = [0x50]; // HAT EEPROM standard address

    // Поддержка чтения больших EEPROM - по умолчанию буфер 32 КБ для обнаружения
    // Можно задать другой размер через переменную окружения EHATROM_BUFFER_SIZE
    let read_len = match std::env::var("EHATROM_BUFFER_SIZE") {
        Ok(size_str) => match size_str.parse::<usize>() {
            Ok(size) => {
                if size < 1024 {
                    println!("Warning: Buffer size too small, using minimum 1KB");
                    1024
                } else {
                    println!("Using custom buffer size: {} bytes", size);
                    size
                }
            }
            Err(_) => {
                println!("Warning: Failed to parse EHATROM_BUFFER_SIZE, using 32KB");
                32 * 1024
            }
        },
        Err(_) => 32 * 1024, // 32 КБ по умолчанию
    };

    let mut found_any = false;

    for device in &devices {
        println!("=== Scanning {} ===", device);
        match detect_and_show_eeprom_info(device, &possible_addrs, read_len) {
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
pub fn find_i2c_devices() -> Vec<String> {
    Vec::new()
}

#[cfg(not(all(feature = "linux", any(target_os = "linux", target_os = "android"))))]
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
pub fn detect_and_show_eeprom_info(
    _dev_path: &str,
    _possible_addrs: &[u16],
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
