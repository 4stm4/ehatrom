//  _  _       _             _  _
// | || |  ___| |_ _ __ ___ | || |
// | || |_/ __| __| '_ ` _ \| || |_
// |__   _\__ | |_| | | | | |__   _|
//   |_| |___/\__|_|_|_| |_|  |_|
//! # ehatrom — EEPROM HAT library for Raspberry Pi HATs
//! - [Documentation (docs.rs)](https://docs.rs/ehatrom)
//! - [GitHub](https://github.com/4stm4/ehatrom)
//!
fn main() {
    // Import I2C functions only on Linux
    #[cfg(not(all(target_os = "linux", feature = "linux")))]
    use ehatrom::Eeprom;
    #[cfg(all(target_os = "linux", feature = "linux"))]
    use ehatrom::{Eeprom, read_from_eeprom_i2c, write_to_eeprom_i2c};
    use std::env;
    use std::process;

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: ehatrom <read|write|make|show|dump|verify|detect> [options]");
        eprintln!("Commands:");
        eprintln!(
            "  read [i2c-dev] <output.bin>             Read HAT EEPROM via I2C and save to file"
        );
        eprintln!(
            "  write [i2c-dev] <input.bin>             Write HAT EEPROM from file to I2C device"
        );
        eprintln!(
            "  make <settings.txt> <output.bin>        Build a HAT EEPROM image from settings.txt"
        );
        eprintln!("  show <input.bin>                        Show parsed EEPROM info (debug)");
        eprintln!("  dump <input.bin>                        Show parsed EEPROM info (readable)");
        eprintln!("  verify <input.bin>                      Check every per-atom CRC-16");
        eprintln!(
            "  detect [i2c-dev]                        Auto-detect HAT EEPROM on specific device"
        );
        eprintln!("  detect --all                            Scan all I2C devices for HAT EEPROM");
        eprintln!("Notes:");
        eprintln!("  HAT EEPROM always uses address 0x50 (automatic)");
        eprintln!("  Default I2C device is /dev/i2c-0 (HAT standard)");
        eprintln!("  Default buffer size is 32KB, customize with EHATROM_BUFFER_SIZE env variable");
        eprintln!("Examples:");
        eprintln!("  ehatrom make settings.txt hat.bin       # Build image from settings.txt");
        eprintln!("  ehatrom dump hat.bin                    # Human-readable dump + CRC check");
        eprintln!("  ehatrom verify hat.bin                  # Exit non-zero on CRC mismatch");
        eprintln!("  sudo ehatrom read hat_data.bin          # Read from /dev/i2c-0 to file");
        eprintln!("  sudo ehatrom write hat_data.bin         # Write from file to /dev/i2c-0");
        eprintln!("  sudo ehatrom read /dev/i2c-1 hat.bin    # Read from specific I2C device");
        eprintln!("  EHATROM_BUFFER_SIZE=1048576 sudo ehatrom read big.bin  # Read 1MB EEPROM");
        eprintln!("  sudo ehatrom detect                     # Scan /dev/i2c-0 (HAT standard)");
        eprintln!("  sudo ehatrom detect --all               # Scan all I2C devices");
        eprintln!("  sudo ehatrom detect /dev/i2c-1          # Scan specific device");
        process::exit(1);
    }
    match args[1].as_str() {
        "read" => {
            // ehatrom read [i2c-dev] <output.bin>
            if args.len() < 3 || args.len() > 4 {
                eprintln!("Usage: ehatrom read [i2c-dev] <output.bin>");
                eprintln!("  Default I2C device: /dev/i2c-0");
                eprintln!("  HAT EEPROM address: 0x50 (automatic)");
                process::exit(1);
            }
            #[cfg(all(target_os = "linux", feature = "linux"))]
            {
                let (dev, output_file) = if args.len() == 3 {
                    // ehatrom read <output.bin>
                    ("/dev/i2c-0", &args[2])
                } else {
                    // ehatrom read <i2c-dev> <output.bin>
                    (args[2].as_str(), &args[3])
                };
                let addr = 0x50u16; // HAT EEPROM fixed address

                // Support reading large EEPROMs - default buffer is 32 KB
                // You can override the size with the EHATROM_BUFFER_SIZE environment variable
                let buf_size = match env::var("EHATROM_BUFFER_SIZE") {
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
                    Err(_) => 32 * 1024, // 32 KB by default
                };

                let buf = vec![0u8; buf_size];
                let mut buf = buf; // for compatibility with function signature
                match read_from_eeprom_i2c(&mut buf, dev, addr, 0) {
                    Ok(()) => {
                        if let Err(e) = std::fs::write(output_file, &buf) {
                            eprintln!("Failed to write output: {e}");
                            process::exit(1);
                        }
                        println!(
                            "HAT EEPROM read from {} (0x50) and saved to {} ({} bytes buffer used)",
                            dev, output_file, buf_size
                        );
                        println!(
                            "Note: Set EHATROM_BUFFER_SIZE env variable if you need a different buffer size"
                        );
                    }
                    Err(e) => {
                        eprintln!("Read error: {e}");
                        process::exit(1);
                    }
                }
            }
            #[cfg(not(feature = "linux"))]
            {
                eprintln!("I2C read requires --features=linux");
                eprintln!("Please rebuild with: cargo build --features linux");
                process::exit(1);
            }
        }
        "write" => {
            // ehatrom write [i2c-dev] <input.bin>
            if args.len() < 3 || args.len() > 4 {
                eprintln!("Usage: ehatrom write [i2c-dev] <input.bin>");
                eprintln!("  Default I2C device: /dev/i2c-0");
                eprintln!("  HAT EEPROM address: 0x50 (automatic)");
                process::exit(1);
            }
            #[cfg(all(target_os = "linux", feature = "linux"))]
            {
                let (dev, input_file) = if args.len() == 3 {
                    // ehatrom write <input.bin>
                    ("/dev/i2c-0", &args[2])
                } else {
                    // ehatrom write <i2c-dev> <input.bin>
                    (args[2].as_str(), &args[3])
                };
                let addr = 0x50u16; // HAT EEPROM fixed address
                let data = match std::fs::read(input_file) {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("Failed to read input: {e}");
                        process::exit(1);
                    }
                };
                match write_to_eeprom_i2c(&data, dev, addr) {
                    Ok(()) => {
                        println!("HAT EEPROM written from {} to {} (0x50)", input_file, dev);
                    }
                    Err(e) => {
                        eprintln!("Write error: {e}");
                        process::exit(1);
                    }
                }
            }
            #[cfg(not(feature = "linux"))]
            {
                eprintln!("I2C write requires --features=linux");
                eprintln!("Please rebuild with: cargo build --features linux");
                process::exit(1);
            }
        }
        "show" => {
            // ehatrom show <input.bin>
            if args.len() != 3 {
                eprintln!("Usage: ehatrom show <input.bin>");
                process::exit(1);
            }
            let data = match std::fs::read(&args[2]) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Failed to read input: {e}");
                    process::exit(1);
                }
            };
            #[cfg(feature = "alloc")]
            match Eeprom::from_bytes(&data) {
                Ok(eeprom) => {
                    println!("EEPROM info:\n{eeprom:#?}");
                }
                Err(e) => {
                    eprintln!("Parse error: {e}");
                    process::exit(1);
                }
            }
            #[cfg(not(feature = "alloc"))]
            {
                eprintln!("Parse command requires 'alloc' feature");
                process::exit(1);
            }
        }
        "dump" => {
            // ehatrom dump <input.bin>
            if args.len() != 3 {
                eprintln!("Usage: ehatrom dump <input.bin>");
                process::exit(1);
            }
            let data = match std::fs::read(&args[2]) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Failed to read input: {e}");
                    process::exit(1);
                }
            };
            #[cfg(feature = "alloc")]
            match Eeprom::from_bytes(&data) {
                Ok(eeprom) => {
                    print!("{eeprom}");
                    match Eeprom::validate(&data) {
                        Ok(()) => println!("\nCRC-16: all atoms valid"),
                        Err(e) => println!("\nCRC-16: {e}"),
                    }
                }
                Err(e) => {
                    eprintln!("Parse error: {e}");
                    process::exit(1);
                }
            }
            #[cfg(not(feature = "alloc"))]
            {
                eprintln!("Dump command requires 'alloc' feature");
                process::exit(1);
            }
        }
        "verify" => {
            // ehatrom verify <input.bin>
            if args.len() != 3 {
                eprintln!("Usage: ehatrom verify <input.bin>");
                process::exit(1);
            }
            let data = match std::fs::read(&args[2]) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Failed to read input: {e}");
                    process::exit(1);
                }
            };
            match Eeprom::validate(&data) {
                Ok(()) => {
                    println!("OK: valid signature and all per-atom CRC-16 checks passed");
                }
                Err(e) => {
                    eprintln!("FAIL: {e}");
                    process::exit(1);
                }
            }
        }
        "make" => {
            // ehatrom make <settings.txt> <output.bin>
            if args.len() != 4 {
                eprintln!("Usage: ehatrom make <settings.txt> <output.bin>");
                process::exit(1);
            }
            #[cfg(feature = "alloc")]
            {
                let settings = match std::fs::read_to_string(&args[2]) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Failed to read settings file: {e}");
                        process::exit(1);
                    }
                };
                match ehatrom::parse_settings(&settings) {
                    Ok(eeprom) => {
                        let bytes = eeprom.serialize();
                        if let Err(e) = std::fs::write(&args[3], &bytes) {
                            eprintln!("Failed to write output: {e}");
                            process::exit(1);
                        }
                        println!(
                            "Wrote {} ({} bytes, {} atoms) from {}",
                            args[3],
                            bytes.len(),
                            eeprom.atom_count(),
                            args[2]
                        );
                    }
                    Err(e) => {
                        eprintln!("{e}");
                        process::exit(1);
                    }
                }
            }
            #[cfg(not(feature = "alloc"))]
            {
                eprintln!("The 'make' command requires the 'alloc' feature");
                process::exit(1);
            }
        }
        "detect" => {
            // ehatrom detect [i2c-dev] or ehatrom detect --all
            #[cfg(feature = "linux")]
            {
                #[cfg(target_os = "linux")]
                {
                    use ehatrom::{detect_all_i2c_devices, detect_and_show_eeprom_info};

                    if args.len() >= 3 && args[2] == "--all" {
                        // Scan all I2C devices
                        match detect_all_i2c_devices() {
                            Ok(()) => {}
                            Err(e) => {
                                eprintln!("Detection error: {e}");
                                process::exit(1);
                            }
                        }
                    } else {
                        // Scan specific device or default
                        let dev = if args.len() >= 3 {
                            &args[2]
                        } else {
                            "/dev/i2c-0" // HAT EEPROM is typically on i2c-0
                        };
                        // Support reading large EEPROMs - default buffer is 32 KB
                        // You can override the size with the EHATROM_BUFFER_SIZE environment variable
                        let read_len = match env::var("EHATROM_BUFFER_SIZE") {
                            Ok(size_str) => match size_str.parse::<usize>() {
                                Ok(size) => {
                                    if size < 1024 {
                                        eprintln!(
                                            "Warning: Buffer size too small, using minimum 1KB"
                                        );
                                        1024
                                    } else {
                                        println!("Using custom buffer size: {} bytes", size);
                                        size
                                    }
                                }
                                Err(_) => {
                                    eprintln!(
                                        "Warning: Failed to parse EHATROM_BUFFER_SIZE, using 32KB"
                                    );
                                    32 * 1024
                                }
                            },
                            Err(_) => 32 * 1024, // 32 KB by default
                        };

                        match detect_and_show_eeprom_info(dev, read_len) {
                            Ok(()) => {}
                            Err(e) => {
                                eprintln!("Detection error: {e}");
                                process::exit(1);
                            }
                        }
                    }
                }
                #[cfg(not(target_os = "linux"))]
                {
                    println!("Linux feature enabled, but running on non-Linux platform.");
                    println!("I2C detection requires actual Linux /dev/i2c-* devices.");
                    println!(
                        "This demonstrates that the library compiles with Linux feature on any platform."
                    );

                    if args.len() >= 3 && args[2] == "--all" {
                        println!("Would scan all I2C devices on Linux");
                    } else {
                        let dev = if args.len() >= 3 {
                            &args[2]
                        } else {
                            "/dev/i2c-0"
                        };
                        println!("Would scan device: {}", dev);
                        println!("Would check addresses: [0x50]");
                    }
                }
            }
            #[cfg(not(feature = "linux"))]
            {
                eprintln!("EEPROM detection requires --features=linux");
                eprintln!("Please rebuild with: cargo build --features linux");
                process::exit(1);
            }
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            eprintln!("Usage: ehatrom <read|write|show|detect> [options]");
            process::exit(1);
        }
    }
}
