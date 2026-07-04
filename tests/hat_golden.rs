#![cfg(feature = "std")]

//! Byte-exact golden test for the official Raspberry Pi HAT EEPROM format.
//!
//! The expected image below was produced by an independent reference
//! implementation of the reference `eepmake`/`eepdump` layout and CRC-16
//! (`raspberrypi/utils`, `eeptools/eeplib.c`): 12-byte header, 8-byte atom
//! headers (`type:u16`, `count:u16`, `dlen:u32`), `dlen = data + 2`, and a
//! per-atom reflected CRC-16 (poly 0x8005) over each atom header and its data.
//!
//! If `ehatrom`'s output ever drifts from this image, it has drifted from the
//! format that a Raspberry Pi bootloader / `eepdump` will accept.

use ehatrom::*;

/// Golden image for the fixture built in [`fixture`].
const GOLDEN: &[u8] = &[
    0x52, 0x2D, 0x50, 0x69, 0x01, 0x00, 0x02, 0x00, 0x69, 0x00, 0x00, 0x00, // header
    // vendor-info atom
    0x01, 0x00, 0x00, 0x00, 0x2D, 0x00, 0x00, 0x00, // type=1 count=0 dlen=45
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
    0x10, // uuid
    0x78, 0x56, // product_id = 0x5678
    0x01, 0x00, // product_ver = 1
    0x0A, 0x0B, // vslen=10 pslen=11
    0x74, 0x65, 0x73, 0x74, 0x76, 0x65, 0x6E, 0x64, 0x6F, 0x72, // "testvendor"
    0x74, 0x65, 0x73, 0x74, 0x70, 0x72, 0x6F, 0x64, 0x75, 0x63, 0x74, // "testproduct"
    0xA9, 0xFF, // crc-16
    // gpio bank0 atom
    0x02, 0x00, 0x01, 0x00, 0x20, 0x00, 0x00, 0x00, // type=2 count=1 dlen=32
    0x05, 0x00, // flags=0x05 power=0x00
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
    0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, // 28 pins
    0x30, 0xA4, // crc-16
];

fn fixture() -> Eeprom {
    let vendor = VendorInfoAtom::new(
        0x5678, // product_id
        1,      // product_ver
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
    let mut eeprom = Eeprom {
        header: EepromHeader::new(),
        vendor_info: vendor,
        gpio_map_bank0: gpio,
        dt_blob: None,
        gpio_map_bank1: None,
        power_supply: None,
        custom_atoms: Vec::new(),
    };
    eeprom.update_header();
    eeprom
}

#[test]
fn serialize_matches_reference_image() {
    let bytes = fixture().serialize();
    assert_eq!(
        bytes, GOLDEN,
        "ehatrom output diverged from the reference HAT image"
    );
}

#[test]
fn header_length_field_matches() {
    let eeprom = fixture();
    let bytes = eeprom.serialize();
    let eeplen = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    assert_eq!(eeplen, bytes.len());
    assert_eq!(eeplen, eeprom.calculate_serialized_size());
}

#[test]
fn every_atom_crc_is_valid() {
    assert!(Eeprom::verify(GOLDEN));
}

#[test]
fn serialize_to_writer_matches_serialize() {
    let eeprom = fixture();
    let mut buf = Vec::new();
    let n = eeprom.serialize_to_writer(&mut buf).unwrap();
    assert_eq!(n, buf.len());
    assert_eq!(buf, GOLDEN);
}

#[test]
fn corrupting_any_atom_byte_fails_verify() {
    let mut bytes = fixture().serialize();
    // Flip a byte inside the gpio atom's data.
    let idx = bytes.len() - 5;
    bytes[idx] ^= 0xFF;
    assert!(!Eeprom::verify(&bytes));
}

#[test]
fn validate_reports_the_failing_atom() {
    // Corrupt the gpio bank0 atom (the second atom, index 1).
    let mut bytes = fixture().serialize();
    let idx = bytes.len() - 5;
    bytes[idx] ^= 0xFF;
    match Eeprom::validate(&bytes) {
        Err(ehatrom::ValidationError::CrcMismatch { atom, .. }) => assert_eq!(atom, 1),
        other => panic!("expected CrcMismatch on atom 1, got {other:?}"),
    }

    // A wrong signature is reported distinctly.
    let mut bad_sig = fixture().serialize();
    bad_sig[0] = 0;
    assert_eq!(
        Eeprom::validate(&bad_sig),
        Err(ehatrom::ValidationError::BadSignature)
    );

    // The untouched golden image validates cleanly.
    assert!(Eeprom::validate(GOLDEN).is_ok());
}

#[test]
fn atom_iterator_walks_the_image() {
    let collected: Vec<_> = ehatrom::atoms(GOLDEN).collect();
    assert_eq!(collected.len(), 2);

    assert_eq!(collected[0].kind(), AtomType::VendorInfo);
    assert_eq!(collected[0].count, 0);
    assert_eq!(collected[0].data.len(), 43); // 22 + "testvendor"(10) + "testproduct"(11)
    assert!(collected[0].crc_valid());

    assert_eq!(collected[1].kind(), AtomType::GpioMapBank0);
    assert_eq!(collected[1].count, 1);
    assert_eq!(collected[1].data.len(), 30);
    assert!(collected[1].crc_valid());

    // A corrupted atom is still yielded, but flagged as invalid.
    let mut bytes = GOLDEN.to_vec();
    let idx = bytes.len() - 5;
    bytes[idx] ^= 0xFF;
    let last = ehatrom::atoms(&bytes).last().unwrap();
    assert!(!last.crc_valid());
}

#[test]
fn roundtrip_from_reference_image() {
    let parsed = Eeprom::from_bytes(GOLDEN).expect("parse golden image");
    assert_eq!(parsed.vendor_info.product_id, 0x5678);
    assert_eq!(parsed.vendor_info.product_ver, 1);
    assert_eq!(&parsed.vendor_info.vendor[..10], b"testvendor");
    assert_eq!(&parsed.vendor_info.product[..11], b"testproduct");
    assert_eq!(parsed.gpio_map_bank0.flags, 0x05);
    assert_eq!(parsed.gpio_map_bank0.pins, [1; 28]);
    // Re-serializing yields the identical image.
    assert_eq!(parsed.serialize(), GOLDEN);
}
