#![cfg(feature = "std")]

use ehatrom::*;

fn make_vendor() -> VendorInfoAtom {
    VendorInfoAtom::new(0x1234, 0x5678, 1, "testvendor", "testproduct", [1; 16])
}

fn make_gpio() -> GpioMapAtom {
    GpioMapAtom {
        flags: 0xAA55,
        pins: [1; 28],
    }
}

fn base_eeprom() -> Eeprom {
    Eeprom {
        header: EepromHeader::new(),
        vendor_info: make_vendor(),
        gpio_map_bank0: make_gpio(),
        dt_blob: None,
        gpio_map_bank1: None,
        custom_atoms: Vec::new(),
    }
}

#[test]
fn test_eeprom_validity() {
    let mut eeprom = base_eeprom();
    assert!(eeprom.is_valid());

    eeprom.header.signature = [0; 4];
    assert!(!eeprom.is_valid());
}

#[test]
fn test_set_version() {
    let mut eeprom = base_eeprom();
    eeprom.set_version(42);
    assert_eq!(eeprom.header.version, 42);
}

#[test]
fn test_add_atoms() {
    let mut eeprom = base_eeprom();
    let initial_atoms = eeprom.header.numatoms;

    eeprom.add_gpio_map_bank1(make_gpio());
    assert!(eeprom.gpio_map_bank1.is_some());
    assert!(eeprom.header.numatoms > initial_atoms);

    eeprom.add_dt_blob(vec![1, 2, 3, 4]);
    assert!(eeprom.dt_blob.is_some());

    eeprom.add_custom_atom(0x80, b"custom".to_vec());
    assert_eq!(eeprom.custom_atoms.len(), 1);
}

#[test]
fn test_serialization_roundtrip() {
    let mut original = base_eeprom();
    original.add_gpio_map_bank1(make_gpio());
    original.add_dt_blob(vec![1, 2, 3]);
    original.add_custom_atom(0x80, b"custom".to_vec());
    original.update_header();

    let bytes = original.serialize_with_crc();
    assert!(Eeprom::verify_crc(&bytes));

    let parsed = Eeprom::from_bytes(&bytes[..bytes.len() - 4]).unwrap();
    assert!(parsed.dt_blob.is_some());
    assert!(parsed.gpio_map_bank1.is_some());
    assert_eq!(parsed.custom_atoms.len(), 1);
}

#[test]
fn test_crc_verification() {
    let eeprom = base_eeprom();
    let mut bytes = eeprom.serialize_with_crc();
    assert!(Eeprom::verify_crc(&bytes));

    bytes[10] ^= 0xFF;
    assert!(!Eeprom::verify_crc(&bytes));
}

#[test]
fn test_invalid_deserialization() {
    assert!(Eeprom::from_bytes(&[1, 2, 3]).is_err());

    let mut invalid = base_eeprom();
    invalid.header.signature = [0; 4];
    assert!(Eeprom::from_bytes(&invalid.serialize()).is_err());
}
