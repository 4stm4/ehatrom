#![cfg(feature = "std")]

//! Randomized round-trip property test (no external dependencies).
//!
//! For many pseudo-random EEPROMs it asserts two properties:
//! 1. `serialize()` always produces an image that `validate()` accepts, and
//! 2. serialization is idempotent through a parse:
//!    `serialize(from_bytes(serialize(x))) == serialize(x)`.
//!
//! The second property is robust to the fixed-buffer string and custom-atom
//! tag quirks, since it compares the canonical serialized forms rather than the
//! in-memory structs.

use ehatrom::*;

/// Tiny deterministic xorshift64 PRNG — avoids pulling in `rand`.
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn range(&mut self, n: u32) -> u32 {
        (self.next_u64() % n as u64) as u32
    }
    fn byte(&mut self) -> u8 {
        self.next_u64() as u8
    }
}

fn random_eeprom(rng: &mut Rng) -> Eeprom {
    // Vendor/product strings as printable ASCII of random length (no interior 0).
    let vlen = rng.range(16) as usize;
    let plen = rng.range(16) as usize;
    let mut vendor = [0u8; 16];
    let mut product = [0u8; 16];
    for b in vendor.iter_mut().take(vlen) {
        *b = b'A' + (rng.range(26) as u8);
    }
    for b in product.iter_mut().take(plen) {
        *b = b'a' + (rng.range(26) as u8);
    }
    let mut uuid = [0u8; 16];
    for b in uuid.iter_mut() {
        *b = rng.byte();
    }
    let vendor_info = VendorInfoAtom {
        uuid,
        product_id: (rng.next_u64() as u16),
        product_ver: (rng.next_u64() as u16),
        vendor,
        product,
    };

    let mut pins = [0u8; 28];
    for b in pins.iter_mut() {
        // Use valid encoded pins or "unused".
        *b = if rng.range(2) == 0 {
            UNUSED_PIN
        } else {
            rng.byte()
        };
    }
    let gpio0 = GpioMapAtom {
        flags: rng.byte(),
        power: rng.byte() & 0x03,
        pins,
    };

    let dt_blob = if rng.range(2) == 0 {
        // Non-empty, otherwise an empty DT atom would be dropped on parse.
        let n = 1 + rng.range(40) as usize;
        Some((0..n).map(|_| rng.byte()).collect::<Vec<u8>>())
    } else {
        None
    };

    let gpio_map_bank1 = if rng.range(2) == 0 {
        let mut p = [0u8; 28];
        for b in p.iter_mut().take(18) {
            *b = rng.byte();
        }
        Some(GpioMapAtom {
            flags: rng.byte(),
            power: rng.byte() & 0x03,
            pins: p,
        })
    } else {
        None
    };

    let power_supply = if rng.range(2) == 0 {
        Some(rng.next_u64() as u32)
    } else {
        None
    };

    let mut custom_atoms = Vec::new();
    for _ in 0..rng.range(4) {
        let n = rng.range(20) as usize;
        custom_atoms.push((0u8, (0..n).map(|_| rng.byte()).collect::<Vec<u8>>()));
    }

    let mut eeprom = Eeprom {
        header: EepromHeader::new(),
        vendor_info,
        gpio_map_bank0: gpio0,
        dt_blob,
        gpio_map_bank1,
        power_supply,
        custom_atoms,
    };
    eeprom.update_header();
    eeprom
}

#[test]
fn randomized_roundtrip_is_idempotent_and_valid() {
    let mut rng = Rng(0x9E3779B97F4A7C15);
    for _ in 0..1000 {
        let eeprom = random_eeprom(&mut rng);

        let b1 = eeprom.serialize();
        assert!(
            Eeprom::validate(&b1).is_ok(),
            "serialized image failed validation: {:?}",
            Eeprom::validate(&b1)
        );
        // Header length field must match the actual length.
        let eeplen = u32::from_le_bytes([b1[8], b1[9], b1[10], b1[11]]) as usize;
        assert_eq!(eeplen, b1.len());

        let parsed = Eeprom::from_bytes(&b1).expect("parse own output");
        let b2 = parsed.serialize();
        assert_eq!(b1, b2, "serialization not idempotent through a parse");
    }
}
