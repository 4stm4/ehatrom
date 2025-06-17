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
        eprintln!("Usage: ehatrom <read|write|show|detect> [options]");
        eprintln!("Commands:");
        eprintln!(
            "  read [i2c-dev] <output.bin>             Read HAT EEPROM via I2C and save to file"
        );
        eprintln!(
            "  write [i2c-dev] <input.bin>             Write HAT EEPROM from file to I2C device"
        );
        eprintln!("  show <input.bin>                        Show parsed EEPROM info from file");
        eprintln!(
            "  detect [i2c-dev]                        Auto-detect HAT EEPROM on specific device"
        );
        eprintln!("  detect --all                            Scan all I2C devices for HAT EEPROM");
        eprintln!("Notes:");
        eprintln!("  HAT EEPROM always uses address 0x50 (automatic)");
        eprintln!("  Default I2C device is /dev/i2c-0 (HAT standard)");
        eprintln!("Examples:");
        eprintln!("  sudo ehatrom read hat_data.bin          # Read from /dev/i2c-0 to file");
        eprintln!("  sudo ehatrom write hat_data.bin         # Write from file to /dev/i2c-0");
        eprintln!("  sudo ehatrom read /dev/i2c-1 hat.bin    # Read from specific I2C device");
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
                let buf = vec![0u8; 256];
                let mut buf = buf; // for compatibility with function signature
                match read_from_eeprom_i2c(&mut buf, dev, addr, 0) {
                    Ok(()) => {
                        if let Err(e) = std::fs::write(output_file, &buf) {
                            eprintln!("Failed to write output: {e}");
                            process::exit(1);
                        }
                        println!(
                            "HAT EEPROM read from {} (0x50) and saved to {}",
                            dev, output_file
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
                        let possible_addrs = &[0x50]; // HAT EEPROM is always at 0x50
                        let read_len = 1024; // read up to 1KB
                        match detect_and_show_eeprom_info(dev, possible_addrs, read_len) {
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
