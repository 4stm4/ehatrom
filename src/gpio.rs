//  _  _       _             _  _
// | || |  ___| |_ _ __ ___ | || |
// | || |_/ __| __| '_ ` _ \| || |_
// |__   _\__ | |_| | | | | |__   _|
//   |_| |___/\__|_|_|_| |_|  |_|
//! # ehatrom — EEPROM HAT library for Raspberry Pi HATs
//! - [Documentation (docs.rs)](https://docs.rs/ehatrom)
//! - [GitHub](https://github.com/4stm4/ehatrom)
//!
//! ## Typed GPIO pin encoding
//!
//! Each byte in a GPIO map atom's `pins` array encodes one pin, following the
//! reference `eepmake` `setgpio` layout:
//!
//! | bits  | meaning                                            |
//! |-------|----------------------------------------------------|
//! | `2:0` | `func_sel` (BCM FSEL values)                        |
//! | `4:3` | reserved (0)                                        |
//! | `6:5` | pull: 0=default, 1=up, 2=down, 3=none               |
//! | `7`   | "board uses this pin" flag                          |
//!
//! A byte of `0x00` therefore means **the board does not use this pin**, not
//! "input" — a used input pin with the default pull is `0x80`. Use
//! [`encode_pin`] / [`decode_pin`] rather than hand-writing these bytes.

const PIN_USED: u8 = 1 << 7;
const FUNC_MASK: u8 = 0b0000_0111;
const PULL_SHIFT: u8 = 5;
const PULL_MASK: u8 = 0b0110_0000;

/// GPIO pin function (`func_sel`), using the BCM2835 FSEL field values.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinFunc {
    Input = 0b000,
    Output = 0b001,
    Alt0 = 0b100,
    Alt1 = 0b101,
    Alt2 = 0b110,
    Alt3 = 0b111,
    Alt4 = 0b011,
    Alt5 = 0b010,
}

impl PinFunc {
    /// Decodes the `func_sel` from a pin byte.
    pub fn from_bits(byte: u8) -> Self {
        match byte & FUNC_MASK {
            0b000 => PinFunc::Input,
            0b001 => PinFunc::Output,
            0b100 => PinFunc::Alt0,
            0b101 => PinFunc::Alt1,
            0b110 => PinFunc::Alt2,
            0b111 => PinFunc::Alt3,
            0b011 => PinFunc::Alt4,
            _ => PinFunc::Alt5, // 0b010
        }
    }
}

/// GPIO pin pull setting.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinPull {
    Default = 0,
    Up = 1,
    Down = 2,
    None = 3,
}

impl PinPull {
    /// Decodes the pull setting from a pin byte.
    pub fn from_bits(byte: u8) -> Self {
        match (byte & PULL_MASK) >> PULL_SHIFT {
            0 => PinPull::Default,
            1 => PinPull::Up,
            2 => PinPull::Down,
            _ => PinPull::None,
        }
    }
}

/// A decoded pin configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinConfig {
    /// Whether the board uses this pin (bit 7).
    pub used: bool,
    /// Pin function.
    pub func: PinFunc,
    /// Pull setting.
    pub pull: PinPull,
}

/// The pin byte for a GPIO the board does not use.
pub const UNUSED_PIN: u8 = 0;

/// Encodes a used pin byte, matching `eepmake`'s `setgpio`.
///
/// The "board uses this pin" flag (bit 7) is always set, so the result is never
/// `0x00`; use [`UNUSED_PIN`] for pins the board leaves alone.
pub fn encode_pin(func: PinFunc, pull: PinPull) -> u8 {
    PIN_USED | (func as u8) | ((pull as u8) << PULL_SHIFT)
}

/// Decodes a pin byte into its [`PinConfig`].
pub fn decode_pin(byte: u8) -> PinConfig {
    PinConfig {
        used: byte & PIN_USED != 0,
        func: PinFunc::from_bits(byte),
        pull: PinPull::from_bits(byte),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_eepmake_bytes() {
        // eepmake always sets bit 7 for a configured pin.
        assert_eq!(encode_pin(PinFunc::Input, PinPull::Default), 0x80);
        assert_eq!(encode_pin(PinFunc::Output, PinPull::Default), 0x81);
        // OUTPUT + UP: 0x80 | 0x01 | (1 << 5) = 0xA1
        assert_eq!(encode_pin(PinFunc::Output, PinPull::Up), 0xA1);
        // ALT0 + DOWN: 0x80 | 0x04 | (2 << 5) = 0xC4
        assert_eq!(encode_pin(PinFunc::Alt0, PinPull::Down), 0xC4);
        // ALT5 + NONE: 0x80 | 0x02 | (3 << 5) = 0xE2
        assert_eq!(encode_pin(PinFunc::Alt5, PinPull::None), 0xE2);
    }

    #[test]
    fn unused_pin_is_zero_and_not_used() {
        assert_eq!(UNUSED_PIN, 0);
        assert!(!decode_pin(UNUSED_PIN).used);
    }

    #[test]
    fn roundtrip_all_combinations() {
        let funcs = [
            PinFunc::Input,
            PinFunc::Output,
            PinFunc::Alt0,
            PinFunc::Alt1,
            PinFunc::Alt2,
            PinFunc::Alt3,
            PinFunc::Alt4,
            PinFunc::Alt5,
        ];
        let pulls = [PinPull::Default, PinPull::Up, PinPull::Down, PinPull::None];
        for f in funcs {
            for p in pulls {
                let byte = encode_pin(f, p);
                let cfg = decode_pin(byte);
                assert!(cfg.used);
                assert_eq!(cfg.func, f);
                assert_eq!(cfg.pull, p);
            }
        }
    }
}
