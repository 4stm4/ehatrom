//! Coverage for the `no_std` (no `alloc`) code paths.
//!
//! These only exist when the crate is built without the `alloc` feature, so the
//! whole file is gated accordingly. Run with:
//!
//!     cargo test --no-default-features
#![cfg(not(feature = "alloc"))]

use ehatrom::*;

/// Byte-exact HAT image for the fixture below (same as `tests/hat_golden.rs`).
static GOLDEN: [u8; 105] = [
    0x52, 0x2D, 0x50, 0x69, 0x01, 0x00, 0x02, 0x00, 0x69, 0x00, 0x00, 0x00, // header
    0x01, 0x00, 0x00, 0x00, 0x2D, 0x00, 0x00, 0x00, // vendor atom header
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
    0x10, // uuid
    0x78, 0x56, // product_id
    0x01, 0x00, // product_ver
    0x0A, 0x0B, // vslen pslen
    0x74, 0x65, 0x73, 0x74, 0x76, 0x65, 0x6E, 0x64, 0x6F, 0x72, // "testvendor"
    0x74, 0x65, 0x73, 0x74, 0x70, 0x72, 0x6F, 0x64, 0x75, 0x63, 0x74, // "testproduct"
    0xA9, 0xFF, // vendor crc
    0x02, 0x00, 0x01, 0x00, 0x20, 0x00, 0x00, 0x00, // gpio atom header
    0x05, 0x00, // flags power
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, // 28 pins
    0x30, 0xA4, // gpio crc
];

fn fixture() -> Eeprom {
    let vendor = VendorInfoAtom::new(
        0x5678,
        1,
        "testvendor",
        "testproduct",
        [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E,
            0x0F, 0x10,
        ],
    );
    let gpio = GpioMapAtom {
        flags: 0x05,
        power: 0x00,
        pins: [1; 28],
    };
    let mut e = Eeprom {
        header: EepromHeader::new(),
        vendor_info: vendor,
        gpio_map_bank0: gpio,
        dt_blob: None,
        gpio_map_bank1: None,
        power_supply: None,
        custom_atoms: &[],
    };
    e.update_header();
    e
}

#[test]
fn serialize_to_slice_matches_golden() {
    let e = fixture();
    let mut buf = [0u8; 256];
    let n = e.serialize_to_slice(&mut buf).unwrap();
    assert_eq!(&buf[..n], &GOLDEN[..]);
    assert!(Eeprom::verify(&buf[..n]));
}

#[test]
fn serialize_to_slice_buffer_too_small() {
    let e = fixture();
    let mut tiny = [0u8; 8];
    assert_eq!(
        e.serialize_to_slice(&mut tiny),
        Err(EhatromError::BufferTooSmall)
    );
}

#[test]
fn from_bytes_no_alloc_parses_golden() {
    let e = Eeprom::from_bytes_no_alloc(&GOLDEN).unwrap();
    let (pid, pver) = (e.vendor_info.product_id, e.vendor_info.product_ver);
    assert_eq!(pid, 0x5678);
    assert_eq!(pver, 1);
    assert_eq!(&e.vendor_info.vendor[..10], b"testvendor");
    assert_eq!(e.gpio_map_bank0.flags, 0x05);
    assert_eq!(e.gpio_map_bank0.pins, [1u8; 28]);
}

#[test]
fn atoms_iterator_works_without_alloc() {
    let count = ehatrom::atoms(&GOLDEN).filter(|a| a.crc_valid()).count();
    assert_eq!(count, 2);
}
