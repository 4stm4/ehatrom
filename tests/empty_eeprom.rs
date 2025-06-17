#![cfg(feature = "std")]

/*
  4STM4
  ocultum
*/
use ehatrom::*;

#[test]
fn test_empty_eeprom_is_invalid() {
    let empty = EepromHeader {
        signature: [0u8; 4],
        version: 0,
        reserved: 0,
        numatoms: 0,
        eeplen: 0,
    };
    let eeprom = Eeprom {
        header: empty,
        vendor_info: VendorInfoAtom {
            vendor_id: 0,
            product_id: 0,
            product_ver: 0,
            vendor: [0; 16],
            product: [0; 16],
            uuid: [0; 16],
        },
        gpio_map_bank0: GpioMapAtom {
            flags: 0,
            pins: [0; 28],
        },
        dt_blob: None,
        gpio_map_bank1: None,
        custom_atoms: vec![],
    };
    assert!(!eeprom.is_valid());
}
