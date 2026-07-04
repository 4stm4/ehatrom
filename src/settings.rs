//  _  _       _             _  _
// | || |  ___| |_ _ __ ___ | || |
// | || |_/ __| __| '_ ` _ \| || |_
// |__   _\__ | |_| | | | | |__   _|
//   |_| |___/\__|_|_|_| |_|  |_|
//! # ehatrom — EEPROM HAT library for Raspberry Pi HATs
//! - [Documentation (docs.rs)](https://docs.rs/ehatrom)
//! - [GitHub](https://github.com/4stm4/ehatrom)
//!
//! ## `eeprom_settings.txt` parser
//!
//! Builds an [`Eeprom`](crate::Eeprom) from the same text settings format that
//! the reference `eepmake` consumes. Supported directives:
//!
//! - `product_uuid <uuid>` — 128-bit UUID; packed into the vendor atom exactly
//!   as `eepmake` does (per-word little-endian). An all-zero UUID is kept as
//!   zero here (this parser never touches `/dev/urandom`).
//! - `product_id <hex>`, `product_ver <hex>`
//! - `vendor "<string>"`, `product "<string>"`
//! - `current_supply <mA>` — power-supply atom (`0x0006`)
//! - `gpio_drive`, `gpio_slew`, `gpio_hysteresis`, `back_power` (bank0 flags)
//! - `bank1_gpio_drive`, `bank1_gpio_slew`, `bank1_gpio_hysteresis` (bank1 flags)
//! - `setgpio <pin> <FUNC> <PULL>` — pins 0..27 go to bank0, 28..45 to bank1
//!
//! Lines beginning with `#` and blank lines are ignored, as are unknown
//! directives (matching `eepmake`'s lenient behaviour). Device-tree and custom
//! file includes are **not** handled (they reference external files); add those
//! atoms via the [`Eeprom`](crate::Eeprom) API after parsing.

use crate::{Eeprom, EepromHeader, GpioMapAtom, PinFunc, PinPull, VendorInfoAtom, encode_pin};
use alloc::vec::Vec;

/// Error returned by [`parse_settings`], carrying the 1-based line number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsError {
    /// 1-based line number where parsing failed.
    pub line: usize,
    /// Human-readable reason.
    pub reason: &'static str,
}

impl core::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "settings error on line {}: {}", self.line, self.reason)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SettingsError {}

/// Parses `eepmake`-style settings text into an [`Eeprom`].
pub fn parse_settings(input: &str) -> Result<Eeprom, SettingsError> {
    let mut vendor_info = VendorInfoAtom {
        uuid: [0u8; 16],
        product_id: 0,
        product_ver: 0,
        vendor: [0u8; 16],
        product: [0u8; 16],
    };
    let mut gpio0 = GpioMapAtom {
        flags: 0,
        power: 0,
        pins: [0u8; 28],
    };
    let mut bank1: Option<GpioMapAtom> = None;
    let mut power_supply: Option<u32> = None;

    for (idx, raw) in input.lines().enumerate() {
        let line = idx + 1;
        let err = |reason| SettingsError { line, reason };

        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let directive = trimmed.split_whitespace().next().unwrap_or("");
        // Remainder after the directive keyword (for quoted-string values).
        let rest = trimmed[directive.len()..].trim();

        match directive {
            "product_uuid" => {
                vendor_info.uuid = parse_uuid(rest).ok_or(err("invalid product_uuid"))?;
            }
            "product_id" => {
                vendor_info.product_id =
                    parse_hex_u16(rest).ok_or(err("invalid product_id (expected hex u16)"))?;
            }
            "product_ver" => {
                vendor_info.product_ver =
                    parse_hex_u16(rest).ok_or(err("invalid product_ver (expected hex u16)"))?;
            }
            "vendor" => {
                copy_str(&mut vendor_info.vendor, unquote(rest));
            }
            "product" => {
                copy_str(&mut vendor_info.product, unquote(rest));
            }
            "current_supply" => {
                let ma: u32 = rest
                    .split_whitespace()
                    .next()
                    .and_then(|t| t.parse().ok())
                    .ok_or(err("invalid current_supply (expected integer mA)"))?;
                power_supply = Some(ma);
            }
            "gpio_drive" => {
                let v = parse_nibble(rest, 8).ok_or(err("invalid gpio_drive"))?;
                gpio0.flags |= v;
            }
            "gpio_slew" => {
                let v = parse_nibble(rest, 2).ok_or(err("invalid gpio_slew"))?;
                gpio0.flags |= v << 4;
            }
            "gpio_hysteresis" => {
                let v = parse_nibble(rest, 2).ok_or(err("invalid gpio_hysteresis"))?;
                gpio0.flags |= v << 6;
            }
            "back_power" => {
                let v = parse_nibble(rest, 2).ok_or(err("invalid back_power"))?;
                gpio0.power = v;
            }
            "bank1_gpio_drive" => {
                let v = parse_nibble(rest, 8).ok_or(err("invalid bank1_gpio_drive"))?;
                bank1_mut(&mut bank1).flags |= v;
            }
            "bank1_gpio_slew" => {
                let v = parse_nibble(rest, 2).ok_or(err("invalid bank1_gpio_slew"))?;
                bank1_mut(&mut bank1).flags |= v << 4;
            }
            "bank1_gpio_hysteresis" => {
                let v = parse_nibble(rest, 2).ok_or(err("invalid bank1_gpio_hysteresis"))?;
                bank1_mut(&mut bank1).flags |= v << 6;
            }
            "setgpio" => {
                let mut it = rest.split_whitespace();
                let pin: usize = it
                    .next()
                    .and_then(|t| t.parse().ok())
                    .ok_or(err("setgpio: invalid pin number"))?;
                let func = it
                    .next()
                    .and_then(parse_func)
                    .ok_or(err("setgpio: invalid function"))?;
                let pull = it
                    .next()
                    .and_then(parse_pull)
                    .ok_or(err("setgpio: invalid pull"))?;
                let byte = encode_pin(func, pull);
                if pin < 28 {
                    gpio0.pins[pin] = byte;
                } else if pin < 46 {
                    bank1_mut(&mut bank1).pins[pin - 28] = byte;
                } else {
                    return Err(err("setgpio: pin number out of range (0..45)"));
                }
            }
            _ => {
                // Unknown directive: ignore, like eepmake.
            }
        }
    }

    let mut eeprom = Eeprom {
        header: EepromHeader::new(),
        vendor_info,
        gpio_map_bank0: gpio0,
        dt_blob: None,
        gpio_map_bank1: bank1,
        power_supply,
        custom_atoms: Vec::new(),
    };
    eeprom.update_header();
    Ok(eeprom)
}

/// Lazily creates the bank1 GPIO map and returns a mutable reference to it.
fn bank1_mut(slot: &mut Option<GpioMapAtom>) -> &mut GpioMapAtom {
    slot.get_or_insert(GpioMapAtom {
        flags: 0,
        power: 0,
        pins: [0u8; 28],
    })
}

/// Copies a string into a fixed 16-byte buffer (truncating, zero-padded).
fn copy_str(buf: &mut [u8; 16], s: &str) {
    *buf = [0u8; 16];
    let bytes = s.as_bytes();
    let n = bytes.len().min(16);
    buf[..n].copy_from_slice(&bytes[..n]);
}

/// Strips a single pair of surrounding double quotes, if present.
fn unquote(s: &str) -> &str {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// Parses the first token as a hexadecimal `u16`.
fn parse_hex_u16(s: &str) -> Option<u16> {
    let tok = s.split_whitespace().next()?;
    let tok = tok.strip_prefix("0x").unwrap_or(tok);
    u16::from_str_radix(tok, 16).ok()
}

/// Parses the first token as a single hex nibble, rejecting values above `max`.
fn parse_nibble(s: &str, max: u8) -> Option<u8> {
    let tok = s.split_whitespace().next()?;
    let tok = tok.strip_prefix("0x").unwrap_or(tok);
    let v = u8::from_str_radix(tok, 16).ok()?;
    if v > max { None } else { Some(v) }
}

fn parse_func(tok: &str) -> Option<PinFunc> {
    Some(match tok {
        "INPUT" => PinFunc::Input,
        "OUTPUT" => PinFunc::Output,
        "ALT0" => PinFunc::Alt0,
        "ALT1" => PinFunc::Alt1,
        "ALT2" => PinFunc::Alt2,
        "ALT3" => PinFunc::Alt3,
        "ALT4" => PinFunc::Alt4,
        "ALT5" => PinFunc::Alt5,
        _ => return None,
    })
}

fn parse_pull(tok: &str) -> Option<PinPull> {
    Some(match tok {
        "DEFAULT" => PinPull::Default,
        "UP" => PinPull::Up,
        "DOWN" => PinPull::Down,
        "NONE" => PinPull::None,
        _ => return None,
    })
}

/// Parses `AAAAAAAA-BBBB-CCCC-DDDD-EEEEFFFFFFFF` into the 16-byte vendor UUID,
/// packing it into `serial[4]` (per-word little-endian) exactly as `eepmake`.
fn parse_uuid(s: &str) -> Option<[u8; 16]> {
    let tok = s.split_whitespace().next()?;
    let hex: Vec<u8> = tok.bytes().filter(|&b| b != b'-').collect();
    if hex.len() != 32 {
        return None;
    }
    let h = core::str::from_utf8(&hex).ok()?;
    let g0 = u32::from_str_radix(&h[0..8], 16).ok()?;
    let g1 = u32::from_str_radix(&h[8..12], 16).ok()?;
    let g2 = u32::from_str_radix(&h[12..16], 16).ok()?;
    let g3 = u32::from_str_radix(&h[16..20], 16).ok()?;
    let g4hi = u32::from_str_radix(&h[20..24], 16).ok()?;
    let g4lo = u32::from_str_radix(&h[24..32], 16).ok()?;

    let serial = [
        g4lo,              // serial[0]
        (g3 << 16) | g4hi, // serial[1]
        (g1 << 16) | g2,   // serial[2]
        g0,                // serial[3]
    ];
    let mut out = [0u8; 16];
    for (i, word) in serial.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_packing_matches_eepmake() {
        let u = parse_uuid("12345678-9abc-def0-1234-56789abcdef0").unwrap();
        assert_eq!(
            u,
            [
                0xF0, 0xDE, 0xBC, 0x9A, 0x78, 0x56, 0x34, 0x12, 0xF0, 0xDE, 0xBC, 0x9A, 0x78, 0x56,
                0x34, 0x12,
            ]
        );
    }

    #[test]
    fn parses_a_full_settings_file() {
        let text = "\
# sample HAT settings
product_uuid 12345678-9abc-def0-1234-56789abcdef0
product_id 0x0001
product_ver 0x0002
vendor \"ACME\"
product \"Test HAT\"
current_supply 1500
gpio_drive 5
gpio_slew 1
gpio_hysteresis 1
back_power 2
setgpio 4 INPUT UP
setgpio 17 OUTPUT DEFAULT
setgpio 40 ALT0 DOWN
";
        let eeprom = parse_settings(text).unwrap();

        // Copy packed u16 fields to locals before comparing (no unaligned refs).
        let product_id = eeprom.vendor_info.product_id;
        let product_ver = eeprom.vendor_info.product_ver;
        assert_eq!(product_id, 0x0001);
        assert_eq!(product_ver, 0x0002);
        assert_eq!(&eeprom.vendor_info.vendor[..4], b"ACME");
        assert_eq!(&eeprom.vendor_info.product[..8], b"Test HAT");
        assert_eq!(eeprom.power_supply, Some(1500));

        // flags = drive(5) | slew(1<<4) | hysteresis(1<<6) = 0x55
        assert_eq!(eeprom.gpio_map_bank0.flags, 0x05 | (1 << 4) | (1 << 6));
        assert_eq!(eeprom.gpio_map_bank0.power, 2);
        assert_eq!(
            eeprom.gpio_map_bank0.pins[4],
            encode_pin(PinFunc::Input, PinPull::Up)
        );
        assert_eq!(
            eeprom.gpio_map_bank0.pins[17],
            encode_pin(PinFunc::Output, PinPull::Default)
        );

        // pin 40 → bank1 index 12
        let bank1 = eeprom.gpio_map_bank1.expect("bank1 present");
        assert_eq!(
            bank1.pins[40 - 28],
            encode_pin(PinFunc::Alt0, PinPull::Down)
        );

        // Whole image must serialize and verify.
        let bytes = eeprom.serialize();
        assert!(Eeprom::verify(&bytes));
    }

    #[test]
    fn reports_line_number_on_error() {
        let text = "product_id 0x1\nproduct_ver notahex\n";
        let e = parse_settings(text).unwrap_err();
        assert_eq!(e.line, 2);
    }
}
