#![cfg(feature = "std")]

//! Broad coverage of public API surface, error paths, and edge cases that the
//! golden/property tests don't exercise directly.

use ehatrom::utils::crc16::crc16;
use ehatrom::*;

// ---- image-crafting helpers -------------------------------------------------

fn header(numatoms: u16) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(b"R-Pi");
    v.push(1); // version
    v.push(0); // reserved
    v.extend_from_slice(&numatoms.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes()); // eeplen (unused by validate/parse loop)
    v
}

/// Builds one atom (header + data + correct CRC-16).
fn atom(atom_type: u16, count: u16, data: &[u8]) -> Vec<u8> {
    let dlen = (data.len() + 2) as u32;
    let mut v = Vec::new();
    v.extend_from_slice(&atom_type.to_le_bytes());
    v.extend_from_slice(&count.to_le_bytes());
    v.extend_from_slice(&dlen.to_le_bytes());
    v.extend_from_slice(data);
    let crc = crc16(&v);
    v.extend_from_slice(&crc.to_le_bytes());
    v
}

fn image(atoms: &[Vec<u8>]) -> Vec<u8> {
    let mut v = header(atoms.len() as u16);
    let total: usize = 12 + atoms.iter().map(Vec::len).sum::<usize>();
    v[8..12].copy_from_slice(&(total as u32).to_le_bytes());
    for a in atoms {
        v.extend_from_slice(a);
    }
    v
}

fn vendor_data(pid: u16, pver: u16, vendor: &[u8], product: &[u8], uuid: [u8; 16]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&uuid);
    v.extend_from_slice(&pid.to_le_bytes());
    v.extend_from_slice(&pver.to_le_bytes());
    v.push(vendor.len() as u8);
    v.push(product.len() as u8);
    v.extend_from_slice(vendor);
    v.extend_from_slice(product);
    v
}

fn gpio_data(flags: u8, power: u8, pins: &[u8; 28]) -> Vec<u8> {
    let mut v = vec![flags, power];
    v.extend_from_slice(pins);
    v
}

// ---- from_bytes: happy path and structure ----------------------------------

#[test]
fn from_bytes_parses_crafted_vendor_and_gpio() {
    let vd = vendor_data(0xBEEF, 3, b"Vend", b"Prod", [7u8; 16]);
    let gd = gpio_data(0x0A, 0x02, &[0u8; 28]);
    let img = image(&[atom(1, 0, &vd), atom(2, 1, &gd)]);

    let e = Eeprom::from_bytes(&img).unwrap();
    let (pid, pver) = (e.vendor_info.product_id, e.vendor_info.product_ver);
    assert_eq!(pid, 0xBEEF);
    assert_eq!(pver, 3);
    assert_eq!(&e.vendor_info.vendor[..4], b"Vend");
    assert_eq!(&e.vendor_info.product[..4], b"Prod");
    assert_eq!(e.vendor_info.uuid, [7u8; 16]);
    assert_eq!(e.gpio_map_bank0.flags, 0x0A);
    assert_eq!(e.gpio_map_bank0.power, 0x02);
}

#[test]
fn from_bytes_requires_vendor_and_gpio() {
    // Only a GPIO atom → missing vendor.
    let gd = gpio_data(0, 0, &[0u8; 28]);
    let only_gpio = image(&[atom(2, 0, &gd)]);
    assert!(Eeprom::from_bytes(&only_gpio).is_err());

    // Only a vendor atom → missing GPIO bank0.
    let vd = vendor_data(1, 1, b"V", b"P", [0u8; 16]);
    let only_vendor = image(&[atom(1, 0, &vd)]);
    assert!(Eeprom::from_bytes(&only_vendor).is_err());
}

#[test]
fn from_bytes_rejects_short_and_bad_signature() {
    assert!(Eeprom::from_bytes(&[]).is_err());
    assert!(Eeprom::from_bytes(&[0u8; 4]).is_err());
    let mut img = header(0);
    img[0] = b'X';
    assert!(Eeprom::from_bytes(&img).is_err());
}

#[test]
fn dt_blob_and_power_and_bank1_round_trip_via_crafted_image() {
    let vd = vendor_data(1, 1, b"V", b"P", [0u8; 16]);
    let gd = gpio_data(0, 0, &[0u8; 28]);
    let dt = atom(3, 2, b"blob-bytes");
    // bank1: flags, power, 18 pins.
    let mut b1 = vec![0x11, 0x01];
    b1.extend_from_slice(&[0xAB; 18]);
    let bank1 = atom(5, 3, &b1);
    let power = atom(6, 4, &1234u32.to_le_bytes());
    let img = image(&[atom(1, 0, &vd), atom(2, 1, &gd), dt, bank1, power]);

    let e = Eeprom::from_bytes(&img).unwrap();
    assert_eq!(e.dt_blob.as_deref(), Some(&b"blob-bytes"[..]));
    let bank1 = e.gpio_map_bank1.expect("bank1");
    assert_eq!(bank1.flags, 0x11);
    assert_eq!(&bank1.pins[..18], &[0xAB; 18]);
    assert_eq!(&bank1.pins[18..], &[0u8; 10]);
    assert_eq!(e.power_supply, Some(1234));
}

#[test]
fn empty_dt_blob_and_short_power_are_ignored() {
    let vd = vendor_data(1, 1, b"V", b"P", [0u8; 16]);
    let gd = gpio_data(0, 0, &[0u8; 28]);
    let img = image(&[
        atom(1, 0, &vd),
        atom(2, 1, &gd),
        atom(3, 2, b""),       // empty DT blob → dropped
        atom(6, 3, &[0u8; 2]), // power body < 4 bytes → ignored
    ]);
    let e = Eeprom::from_bytes(&img).unwrap();
    assert!(e.dt_blob.is_none());
    assert!(e.power_supply.is_none());
}

#[test]
fn unknown_atom_type_becomes_custom() {
    let vd = vendor_data(1, 1, b"V", b"P", [0u8; 16]);
    let gd = gpio_data(0, 0, &[0u8; 28]);
    let img = image(&[
        atom(1, 0, &vd),
        atom(2, 1, &gd),
        atom(0x00AA, 2, b"weird"), // reserved/unknown type
        atom(4, 3, b"real-custom"),
    ]);
    let e = Eeprom::from_bytes(&img).unwrap();
    assert_eq!(e.custom_atoms.len(), 2);
    assert_eq!(e.custom_atoms[0], b"weird");
    assert_eq!(e.custom_atoms[1], b"real-custom");
}

// ---- validate: every variant ------------------------------------------------

#[test]
fn validate_too_short() {
    assert_eq!(Eeprom::validate(&[0u8; 5]), Err(ValidationError::TooShort));
    assert!(!Eeprom::verify(&[0u8; 5]));
}

#[test]
fn validate_bad_signature() {
    let mut img = header(0);
    img[1] = 0;
    assert_eq!(Eeprom::validate(&img), Err(ValidationError::BadSignature));
}

#[test]
fn validate_truncated_atom_header() {
    // Header claims 1 atom but no atom bytes follow.
    let img = header(1);
    assert_eq!(
        Eeprom::validate(&img),
        Err(ValidationError::TruncatedAtomHeader { atom: 0 })
    );
}

#[test]
fn validate_bad_dlen() {
    let mut img = header(1);
    img.extend_from_slice(&2u16.to_le_bytes()); // type
    img.extend_from_slice(&0u16.to_le_bytes()); // count
    img.extend_from_slice(&0u32.to_le_bytes()); // dlen = 0 (< CRC size)
    assert_eq!(
        Eeprom::validate(&img),
        Err(ValidationError::BadDlen { atom: 0 })
    );
}

#[test]
fn validate_truncated_atom_body() {
    let mut img = header(1);
    img.extend_from_slice(&2u16.to_le_bytes()); // type
    img.extend_from_slice(&0u16.to_le_bytes()); // count
    img.extend_from_slice(&10u32.to_le_bytes()); // dlen=10 → needs 8 data + 2 crc
    // ...but no data follows.
    assert_eq!(
        Eeprom::validate(&img),
        Err(ValidationError::TruncatedAtom { atom: 0 })
    );
}

#[test]
fn validate_crc_mismatch_names_second_atom() {
    let vd = vendor_data(1, 1, b"V", b"P", [0u8; 16]);
    let gd = gpio_data(0, 0, &[0u8; 28]);
    let mut img = image(&[atom(1, 0, &vd), atom(2, 1, &gd)]);
    let last = img.len() - 1;
    img[last] ^= 0xFF; // break the gpio atom's CRC
    match Eeprom::validate(&img) {
        Err(ValidationError::CrcMismatch { atom, .. }) => assert_eq!(atom, 1),
        other => panic!("expected CrcMismatch, got {other:?}"),
    }
}

// ---- serialize: sizing and buffer handling ----------------------------------

fn sample_eeprom() -> Eeprom {
    let mut e = Eeprom {
        header: EepromHeader::new(),
        vendor_info: VendorInfoAtom::new(0x1234, 1, "vendor", "product", [9u8; 16]),
        gpio_map_bank0: GpioMapAtom {
            flags: 1,
            power: 0,
            pins: [0u8; 28],
        },
        dt_blob: None,
        gpio_map_bank1: None,
        power_supply: None,
        custom_atoms: Vec::new(),
    };
    e.update_header();
    e
}

#[test]
fn calculate_size_matches_serialize_len_and_header() {
    let mut e = sample_eeprom();
    e.add_dt_blob(vec![1, 2, 3, 4, 5]);
    e.add_gpio_map_bank1(GpioMapAtom {
        flags: 0,
        power: 0,
        pins: [1u8; 28],
    });
    e.add_power_supply(999);
    e.add_custom_atom(vec![0xEE; 7]);

    let bytes = e.serialize();
    assert_eq!(bytes.len(), e.calculate_serialized_size());
    // Header numatoms and eeplen reflect reality.
    let numatoms = u16::from_le_bytes([bytes[6], bytes[7]]);
    let eeplen = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    assert_eq!(numatoms, e.atom_count());
    assert_eq!(numatoms, 6); // vendor, gpio0, dt, bank1, power, custom
    assert_eq!(eeplen, bytes.len());
    assert!(Eeprom::verify(&bytes));
}

#[test]
fn serialize_into_reports_buffer_too_small() {
    let e = sample_eeprom();
    let mut tiny = [0u8; 8];
    assert_eq!(
        e.serialize_into(&mut tiny),
        Err(EhatromError::BufferTooSmall)
    );
    // Exactly-sized buffer succeeds.
    let mut exact = vec![0u8; e.calculate_serialized_size()];
    assert_eq!(e.serialize_into(&mut exact), Ok(exact.len()));
}

// ---- constructors & mutators ------------------------------------------------

#[test]
fn vendor_new_truncates_long_strings_to_16() {
    let v = VendorInfoAtom::new(
        1,
        1,
        "0123456789ABCDEFGHIJ",
        "abcdefghijklmnopqrstuv",
        [0u8; 16],
    );
    assert_eq!(&v.vendor, b"0123456789ABCDEF");
    assert_eq!(&v.product, b"abcdefghijklmnop");
}

#[test]
fn add_methods_bump_atom_count() {
    let mut e = sample_eeprom();
    assert_eq!(e.atom_count(), 2);
    e.add_dt_blob(vec![1]);
    assert_eq!(e.atom_count(), 3);
    e.add_gpio_map_bank1(GpioMapAtom {
        flags: 0,
        power: 0,
        pins: [0u8; 28],
    });
    assert_eq!(e.atom_count(), 4);
    e.add_power_supply(1);
    assert_eq!(e.atom_count(), 5);
    e.add_custom_atom(vec![1, 2]);
    assert_eq!(e.atom_count(), 6);
    assert_eq!(e.header.numatoms, 6);
}

#[test]
fn is_valid_and_set_version() {
    let mut e = sample_eeprom();
    assert!(e.is_valid());
    e.set_version(0);
    assert!(!e.is_valid());
    e.set_version(2);
    assert!(e.is_valid());
    assert_eq!(e.header.version, 2);
}

#[test]
fn add_vendor_and_gpio_replace_fields() {
    let mut e = sample_eeprom();
    e.add_vendor_info(VendorInfoAtom::new(0xAAAA, 9, "x", "y", [1u8; 16]));
    e.add_gpio_map_bank0(GpioMapAtom {
        flags: 0x7F,
        power: 3,
        pins: [2u8; 28],
    });
    let pid = e.vendor_info.product_id;
    assert_eq!(pid, 0xAAAA);
    assert_eq!(e.gpio_map_bank0.flags, 0x7F);
}

// ---- AtomType mapping -------------------------------------------------------

#[test]
fn atom_type_from_u16() {
    assert_eq!(AtomType::from(1), AtomType::VendorInfo);
    assert_eq!(AtomType::from(2), AtomType::GpioMapBank0);
    assert_eq!(AtomType::from(3), AtomType::DtBlob);
    assert_eq!(AtomType::from(4), AtomType::Custom);
    assert_eq!(AtomType::from(5), AtomType::GpioMapBank1);
    assert_eq!(AtomType::from(6), AtomType::PowerSupply);
    assert_eq!(AtomType::from(0), AtomType::Unknown);
    assert_eq!(AtomType::from(0xFFFF), AtomType::Unknown);
}

// ---- atoms() iterator edges -------------------------------------------------

#[test]
fn atoms_empty_or_bad_signature_yields_nothing() {
    assert_eq!(ehatrom::atoms(&[]).count(), 0);
    assert_eq!(ehatrom::atoms(&[0u8; 20]).count(), 0);
}

#[test]
fn atoms_stops_on_truncation() {
    // Header says 3 atoms, but only one complete atom is present.
    let gd = gpio_data(0, 0, &[0u8; 28]);
    let mut img = header(3);
    img.extend_from_slice(&atom(2, 0, &gd));
    // Only one atom is actually parseable.
    assert_eq!(ehatrom::atoms(&img).count(), 1);
}

// ---- Display smoke tests ----------------------------------------------------

#[test]
fn display_impls_do_not_panic() {
    let mut e = sample_eeprom();
    e.add_dt_blob(vec![1, 2, 3]);
    e.add_gpio_map_bank1(GpioMapAtom {
        flags: 0,
        power: 0,
        pins: [0u8; 28],
    });
    e.add_power_supply(500);
    e.add_custom_atom(vec![0xDE, 0xAD]);

    let s = format!("{e}");
    assert!(s.contains("Vendor Info"));
    assert!(s.contains("GPIO Map Bank0"));
    assert!(s.contains("Power Supply: 500 mA"));
    assert!(s.contains("Custom Atoms"));

    assert!(!format!("{}", e.header).is_empty());
    assert!(!format!("{}", e.vendor_info).is_empty());
    assert!(!format!("{}", e.gpio_map_bank0).is_empty());
}

#[test]
fn error_types_display() {
    assert!(!format!("{}", EhatromError::BufferTooSmall).is_empty());
    assert!(!format!("{}", EhatromError::InvalidCrc).is_empty());
    assert!(
        format!(
            "{}",
            ValidationError::CrcMismatch {
                atom: 2,
                expected: 1,
                found: 2
            }
        )
        .contains("atom 2")
    );
}

// ---- settings: directive coverage ------------------------------------------

#[test]
fn settings_unquoted_vendor_and_defaults() {
    let e = parse_settings("vendor Acme\nproduct Board\n").unwrap();
    assert_eq!(&e.vendor_info.vendor[..4], b"Acme");
    assert_eq!(&e.vendor_info.product[..5], b"Board");
    // Missing numeric fields default to 0; still serializes and verifies.
    assert!(Eeprom::verify(&e.serialize()));
}

#[test]
fn settings_bank1_directives_and_setgpio_ranges() {
    let text = "\
bank1_gpio_drive 3
bank1_gpio_slew 1
bank1_gpio_hysteresis 2
setgpio 0 OUTPUT DEFAULT
setgpio 45 ALT5 NONE
";
    let e = parse_settings(text).unwrap();
    let b1 = e.gpio_map_bank1.expect("bank1 created");
    assert_eq!(b1.flags, 3 | (1 << 4) | (2 << 6));
    assert_eq!(
        e.gpio_map_bank0.pins[0],
        encode_pin(PinFunc::Output, PinPull::Default)
    );
    assert_eq!(b1.pins[45 - 28], encode_pin(PinFunc::Alt5, PinPull::None));
}

#[test]
fn settings_ignores_unknown_and_comments() {
    let e = parse_settings("# a comment\n\nnonsense_directive 1 2 3\nproduct_id 0x9\n").unwrap();
    assert_eq!(e.vendor_info.product_id, 0x9);
}

#[test]
fn settings_errors_are_line_numbered() {
    assert_eq!(parse_settings("gpio_drive 9\n").unwrap_err().line, 1); // 9 > max 8
    assert_eq!(
        parse_settings("\nsetgpio 99 INPUT UP\n").unwrap_err().line,
        2
    ); // pin OOR
    assert_eq!(parse_settings("setgpio 4 BOGUS UP\n").unwrap_err().line, 1); // bad func
    assert_eq!(parse_settings("current_supply abc\n").unwrap_err().line, 1);
}

// ---- CustomAtom<N> ----------------------------------------------------------

#[test]
fn custom_atom_debug_and_display() {
    let c: CustomAtom<3> = CustomAtom {
        atom_type: 0x42,
        data: [1, 2, 3],
    };
    assert_eq!(
        format!("{c:?}"),
        "CustomAtom { atom_type: 0x42, data: [1, 2, 3] }"
    );
    assert_eq!(format!("{c}"), "atom_type: 0x42\ndata: [01, 02, 03]");

    // Copy semantics: the copy formats identically.
    let d = c;
    assert_eq!(format!("{d:?}"), format!("{c:?}"));
}

#[test]
fn custom_atom_zero_length() {
    let c: CustomAtom<0> = CustomAtom {
        atom_type: 0xFF,
        data: [],
    };
    assert_eq!(format!("{c:?}"), "CustomAtom { atom_type: 0xFF, data: [] }");
    assert_eq!(format!("{c}"), "atom_type: 0xFF\ndata: []");
}
