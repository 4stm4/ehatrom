//  _  _       _             _  _
// | || |  ___| |_ _ __ ___ | || |
// | || |_/ __| __| '_ ` _ \| || |_
// |__   _\__ | |_| | | | | |__   _|
//   |_| |___/\__|_|_|_| |_|  |_|
//! # ehatrom — EEPROM HAT library for Raspberry Pi HATs
//! - [Documentation (docs.rs)](https://docs.rs/ehatrom)
//! - [GitHub](https://github.com/4stm4/ehatrom)
//!
//! ## CRC-16 used by the official Raspberry Pi HAT EEPROM format
//!
//! This is a byte-for-byte port of the `getcrc()` routine from the reference
//! `eepmake`/`eepdump` tools (`raspberrypi/utils`, `eeptools/eeplib.c`). It is a
//! reflected CRC-16 with polynomial `0x8005` and initial value `0x0000`
//! (a.k.a. CRC-16/ARC). The reference implementation reads each input byte
//! LSB-first and reverses the final register, so any faithful reimplementation
//! must do the same — otherwise `eepdump` reports a CRC mismatch.
//!
//! Reference check value: `crc16(b"123456789") == 0xBB3D`.

/// CRC-16 polynomial used by the HAT EEPROM tools.
pub const CRC16_POLY: u16 = 0x8005;

/// Incremental HAT-format CRC-16, mirroring `crc_add`/`crc_get` in `eeptools`.
///
/// Feed bytes with [`Crc16::update`] (in any number of calls) and read the
/// result with [`Crc16::finalize`]. The result is identical to [`crc16`] over
/// the concatenation of all fed bytes.
#[derive(Debug, Clone, Copy)]
pub struct Crc16 {
    out: u16,
}

impl Default for Crc16 {
    fn default() -> Self {
        Self::new()
    }
}

impl Crc16 {
    /// Creates a fresh CRC-16 state.
    pub const fn new() -> Self {
        Crc16 { out: 0 }
    }

    /// Feeds more bytes into the running CRC.
    pub fn update(&mut self, data: &[u8]) {
        let mut out = self.out;
        // Feed every bit of every byte, LSB-first (matches `(*data >> bits_read) & 1`).
        for &byte in data {
            for bit in 0..8u32 {
                let bit_flag = out >> 15;
                out <<= 1;
                out |= ((byte >> bit) & 1) as u16;
                if bit_flag != 0 {
                    out ^= CRC16_POLY;
                }
            }
        }
        self.out = out;
    }

    /// Consumes the state and returns the final CRC-16.
    pub fn finalize(self) -> u16 {
        let mut out = self.out;
        // "Push out" the last 16 bits.
        for _ in 0..16 {
            let bit_flag = out >> 15;
            out <<= 1;
            if bit_flag != 0 {
                out ^= CRC16_POLY;
            }
        }
        // Reverse the bits of the register to obtain the final CRC.
        let mut crc: u16 = 0;
        let mut i: u16 = 0x8000;
        let mut j: u16 = 0x0001;
        while i != 0 {
            if out & i != 0 {
                crc |= j;
            }
            i >>= 1;
            j <<= 1;
        }
        crc
    }
}

/// Computes the HAT-format CRC-16 over `data`.
///
/// The algorithm mirrors `getcrc()` in the reference `eeptools`: bits are
/// consumed from the least-significant end of each byte, the register uses
/// polynomial [`CRC16_POLY`] starting from `0`, and the final register is
/// bit-reversed to produce the result.
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc = Crc16::new();
    crc.update(data);
    crc.finalize()
}

#[cfg(test)]
mod tests {
    use super::{Crc16, crc16};

    #[test]
    fn reference_check_value() {
        // Standard CRC-16/ARC check value, also produced by the reference eepmake.
        assert_eq!(crc16(b"123456789"), 0xBB3D);
    }

    #[test]
    fn empty_is_zero() {
        assert_eq!(crc16(&[]), 0);
    }

    #[test]
    fn incremental_matches_one_shot() {
        let data = b"123456789";
        let mut crc = Crc16::new();
        crc.update(&data[..4]);
        crc.update(&data[4..]);
        assert_eq!(crc.finalize(), crc16(data));
        assert_eq!(crc.finalize(), 0xBB3D);
    }
}
