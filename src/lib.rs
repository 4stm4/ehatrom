//  _  _       _             _  _
// | || |  ___| |_ _ __ ___ | || |
// | || |_/ __| __| '_ ` _ \| || |_
// |__   _\__ | |_| | | | | |__   _|
//   |_| |___/\__|_|_|_| |_|  |_|
//! # ehatrom — EEPROM HAT library for Raspberry Pi HATs
//! - [Documentation (docs.rs)](https://docs.rs/ehatrom)
//! - [GitHub](https://github.com/4stm4/ehatrom)
//!
//! `ehatrom` serializes and parses EEPROM images in the **official Raspberry Pi
//! HAT ID EEPROM format**, byte-compatible with the reference `eepmake`/`eepdump`
//! tools (`raspberrypi/utils`, `eeptools`). Concretely this means:
//!
//! * the 12-byte header (`R-Pi`, version, reserved, `numatoms`, `eeplen`);
//! * an 8-byte atom header laid out as `type:u16`, `count:u16`, `dlen:u32`
//!   (all little-endian), where `dlen` is the length of the atom data **plus**
//!   the trailing 2-byte CRC;
//! * a per-atom CRC-16 (poly `0x8005`, reflected — see [`utils::crc16`]) computed
//!   over the atom header and its data;
//! * spec atom types: vendor-info `0x0001`, GPIO map bank0 `0x0002`, device-tree
//!   blob `0x0003`, manufacturer custom `0x0004`, GPIO map bank1 `0x0005`.
//!
//! All multi-byte integers are written/read as little-endian regardless of host
//! endianness, so the output is identical on any platform.

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

use core::fmt;

#[cfg(feature = "alloc")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

/// Custom error type for bare-metal compatibility
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EhatromError {
    /// I2C communication error
    I2cError,
    /// Invalid or corrupted data
    InvalidData,
    /// Invalid per-atom CRC-16 checksum
    InvalidCrc,
    /// Buffer too small for operation
    BufferTooSmall,
    /// Device not found
    DeviceNotFound,
    /// Timeout during operation
    Timeout,
}

impl core::fmt::Display for EhatromError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EhatromError::I2cError => write!(f, "I2C communication error"),
            EhatromError::InvalidData => write!(f, "Invalid or corrupted EEPROM data"),
            EhatromError::InvalidCrc => write!(f, "Invalid CRC-16 checksum"),
            EhatromError::BufferTooSmall => write!(f, "Buffer too small for operation"),
            EhatromError::DeviceNotFound => write!(f, "Device not found"),
            EhatromError::Timeout => write!(f, "Timeout during operation"),
        }
    }
}

// Implement Error trait when std is available
#[cfg(feature = "std")]
impl std::error::Error for EhatromError {}

pub mod gpio;
pub mod utils;
pub use gpio::{PinConfig, PinFunc, PinPull, UNUSED_PIN, decode_pin, encode_pin};
use utils::crc16::crc16;

#[cfg(feature = "alloc")]
pub mod settings;
#[cfg(feature = "alloc")]
pub use settings::{SettingsError, parse_settings};

#[cfg(all(feature = "linux", any(target_os = "linux", target_os = "android")))]
use i2cdev::{core::I2CDevice, linux::LinuxI2CDevice};

/// EEPROM signature: bytes `0x52 0x2D 0x50 0x69` ("R-Pi").
pub const EEPROM_SIGNATURE: [u8; 4] = *b"R-Pi";
/// EEPROM format version emitted by this library.
pub const FORMAT_VERSION: u8 = 1;

/// Size of the fixed EEPROM header in bytes.
const HEADER_SIZE: usize = 12;
/// Size of an atom header (`type` + `count` + `dlen`) in bytes.
const ATOM_HDR_SIZE: usize = 8;
/// Size of the trailing per-atom CRC-16 in bytes.
const CRC_SIZE: usize = 2;
/// Number of GPIO pins described by a bank0 GPIO map atom (GPIO0..GPIO27).
pub const GPIO_COUNT: usize = 28;
/// Number of GPIO pins described by a bank1 GPIO map atom (GPIO28..GPIO45).
pub const GPIO_COUNT_BANK1: usize = 18;
/// On-the-wire size of the fixed vendor-info prefix (before the strings).
const VENDOR_FIXED_SIZE: usize = 22; // uuid(16) + pid(2) + pver(2) + vslen(1) + pslen(1)

/// EEPROM header structure for Raspberry Pi HATs.
///
/// The header is always 12 bytes long. All multi-byte fields are little-endian
/// on the wire; this struct is never serialized by raw pointer cast, so the
/// output is endianness-independent.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default)]
pub struct EepromHeader {
    /// Always 0x52 0x2D 0x50 0x69 ("R-Pi")
    pub signature: [u8; 4],
    /// Format version (0x01 for first version)
    pub version: u8,
    /// Reserved byte (0x00)
    pub reserved: u8,
    /// Number of atoms (Little Endian)
    pub numatoms: u16,
    /// Total length of EEPROM data including the header (Little Endian)
    pub eeplen: u32,
}

impl EepromHeader {
    /// Creates a new EepromHeader with default values
    pub const fn new() -> Self {
        EepromHeader {
            signature: EEPROM_SIGNATURE,
            version: FORMAT_VERSION,
            reserved: 0,
            numatoms: 0,
            eeplen: 0,
        }
    }
}

/// Atom header as defined by the HAT specification.
///
/// On the wire this is 8 bytes: `type:u16`, `count:u16`, `dlen:u32`, all
/// little-endian. `dlen` counts the atom data **plus** the trailing 2-byte
/// CRC-16 (`dlen = data_len + 2`).
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct AtomHeader {
    /// Type identifier of the atom (little endian, e.g. 0x0001 for vendor info)
    pub atom_type: u16,
    /// Incrementing atom index within the EEPROM (little endian, starts at 0)
    pub count: u16,
    /// Length of atom data **and** its 2-byte CRC (little endian)
    pub dlen: u32,
}

/// Atom type identifiers, matching the reference `eeptools` values.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomType {
    VendorInfo = 0x0001,
    GpioMapBank0 = 0x0002,
    DtBlob = 0x0003,
    Custom = 0x0004,
    GpioMapBank1 = 0x0005,
    PowerSupply = 0x0006,
    Unknown = 0xFFFF,
}

/// Vendor-info atom data.
///
/// Wire layout matches `struct vendor_info_d`: `serial[4]` (the 16-byte UUID),
/// `pid:u16`, `pver:u16`, `vslen:u8`, `pslen:u8`, followed by the variable-length
/// vendor and product strings. The strings are stored here in fixed 16-byte
/// buffers; on serialization only the used prefix (up to the first NUL) is
/// written and `vslen`/`pslen` are set accordingly.
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct VendorInfoAtom {
    pub uuid: [u8; 16],    // serial[4], least-significant word first
    pub product_id: u16,   // pid
    pub product_ver: u16,  // pver
    pub vendor: [u8; 16],  // vendor string (unused tail is zero)
    pub product: [u8; 16], // product string (unused tail is zero)
}

impl fmt::Debug for VendorInfoAtom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let product_id = self.product_id;
        let product_ver = self.product_ver;
        let uuid = self.uuid;
        #[cfg(feature = "alloc")]
        {
            let vendor_str = String::from_utf8_lossy(&self.vendor)
                .trim_end_matches('\0')
                .to_string();
            let product_str = String::from_utf8_lossy(&self.product)
                .trim_end_matches('\0')
                .to_string();
            write!(
                f,
                "VendorInfoAtom {{ product_id: {product_id}, product_ver: {product_ver}, vendor: \"{vendor_str}\", product: \"{product_str}\", uuid: {uuid:?} }}"
            )
        }
        #[cfg(not(feature = "alloc"))]
        {
            let vendor = self.vendor;
            let product = self.product;
            write!(
                f,
                "VendorInfoAtom {{ product_id: {product_id}, product_ver: {product_ver}, vendor: {vendor:?}, product: {product:?}, uuid: {uuid:?} }}"
            )
        }
    }
}

impl core::fmt::Display for EepromHeader {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let signature = self.signature;
        let version = self.version;
        let reserved = self.reserved;
        let numatoms = self.numatoms;
        let eeplen = self.eeplen;
        write!(
            f,
            "signature: {signature:?}\nversion: {version}\nreserved: {reserved}\nnumatoms: {numatoms}\neeplen: {eeplen}"
        )
    }
}

impl core::fmt::Display for AtomHeader {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let atom_type = self.atom_type;
        let count = self.count;
        let dlen = self.dlen;
        write!(
            f,
            "atom_type: 0x{atom_type:04X}\ncount: {count}\ndlen: {dlen}",
        )
    }
}

impl core::fmt::Display for VendorInfoAtom {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let product_id = self.product_id;
        let product_ver = self.product_ver;
        let vendor_buf = self.vendor;
        let product_buf = self.product;
        let uuid = self.uuid;

        #[cfg(feature = "alloc")]
        {
            let vendor_string = String::from_utf8_lossy(&vendor_buf);
            let vendor = vendor_string.trim_end_matches('\0');
            let product_string = String::from_utf8_lossy(&product_buf);
            let product = product_string.trim_end_matches('\0');
            write!(
                f,
                "product_id: 0x{product_id:04X}\nproduct_ver: {product_ver}\nvendor: {vendor}\nproduct: {product}\nuuid: {uuid:02X?}"
            )
        }
        #[cfg(not(feature = "alloc"))]
        {
            let vendor_len = vendor_buf
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(vendor_buf.len());
            let product_len = product_buf
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(product_buf.len());

            match (
                core::str::from_utf8(&vendor_buf[..vendor_len]),
                core::str::from_utf8(&product_buf[..product_len]),
            ) {
                (Ok(vendor), Ok(product)) => {
                    write!(
                        f,
                        "product_id: 0x{product_id:04X}\nproduct_ver: {product_ver}\nvendor: {vendor}\nproduct: {product}\nuuid: {uuid:02X?}"
                    )
                }
                _ => {
                    write!(
                        f,
                        "product_id: 0x{product_id:04X}\nproduct_ver: {product_ver}\nvendor: {vendor_buf:02X?}\nproduct: {product_buf:02X?}\nuuid: {uuid:02X?}"
                    )
                }
            }
        }
    }
}

/// GPIO map atom data (`struct gpio_map_d`).
///
/// Wire layout is `flags:u8`, `power:u8`, then one byte per pin. A bank0 atom
/// carries [`GPIO_COUNT`] (28) pin bytes; a bank1 atom carries only
/// [`GPIO_COUNT_BANK1`] (18). Per-pin encoding follows the spec `func_sel`
/// field: `0x00` = input, `0x01` = output, `0x04..0x0B` = ALT0..ALT5.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct GpioMapAtom {
    pub flags: u8,
    pub power: u8,
    pub pins: [u8; GPIO_COUNT],
}

impl core::fmt::Display for GpioMapAtom {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let flags = self.flags;
        let power = self.power;
        let pins = self.pins;
        write!(
            f,
            "flags: 0x{flags:02X}\npower: 0x{power:02X}\npins: {pins:?}"
        )
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DtBlobAtom {
    pub dlen: u32,
}

impl core::fmt::Display for DtBlobAtom {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let dlen = self.dlen;
        write!(f, "dlen: {dlen} (blob data not shown)")
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct CustomAtom<const N: usize> {
    pub atom_type: u8,
    pub data: [u8; N],
}

impl<const N: usize> fmt::Debug for CustomAtom<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let self_ptr = self as *const Self as *const u8;
        let data_offset = core::mem::size_of::<u8>();
        let data_ptr = unsafe { self_ptr.add(data_offset) };
        let mut data = [0u8; N];
        unsafe {
            core::ptr::copy_nonoverlapping(data_ptr, data.as_mut_ptr(), N);
        }
        write!(
            f,
            "CustomAtom {{ atom_type: 0x{:02X}, data: {:?} }}",
            self.atom_type,
            &data[..]
        )
    }
}

impl<const N: usize> core::fmt::Display for CustomAtom<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let atom_type = self.atom_type;
        let data = self.data;
        write!(f, "atom_type: 0x{atom_type:02X}\ndata: {data:02X?}")
    }
}

pub enum EepromAtom {
    VendorInfo(VendorInfoAtom),
    GpioMapBank0(GpioMapAtom),
    #[cfg(feature = "alloc")]
    DtBlob(Vec<u8>),
    #[cfg(not(feature = "alloc"))]
    DtBlob(&'static [u8]),
    GpioMapBank1(GpioMapAtom),
    #[cfg(feature = "alloc")]
    Custom(Vec<u8>, u8),
    #[cfg(not(feature = "alloc"))]
    Custom(&'static [u8], u8),
}

#[derive(Debug, Clone)]
pub struct Eeprom {
    pub header: EepromHeader,
    pub vendor_info: VendorInfoAtom,
    pub gpio_map_bank0: GpioMapAtom,
    #[cfg(feature = "alloc")]
    pub dt_blob: Option<Vec<u8>>, // DT blob can be variable length
    #[cfg(not(feature = "alloc"))]
    pub dt_blob: Option<&'static [u8]>, // Static data for no_std
    pub gpio_map_bank1: Option<GpioMapAtom>, // Optional
    /// Optional power-supply atom (`0x0006`): required back-power current in mA.
    pub power_supply: Option<u32>,
    /// Manufacturer custom atoms. Each entry is `(tag, data)`; the `tag` is an
    /// informational identifier only — on the wire every custom atom is emitted
    /// with the spec type `0x0004`, and parsing reports `0x04` as the tag.
    #[cfg(feature = "alloc")]
    pub custom_atoms: Vec<(u8, Vec<u8>)>,
    #[cfg(not(feature = "alloc"))]
    pub custom_atoms: &'static [(u8, &'static [u8])], // Static data for no_std
}

/// Returns the used length of a fixed string buffer (up to the first NUL).
fn string_len(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

impl VendorInfoAtom {
    /// Encodes the vendor-info atom body into `out`, returning its length.
    ///
    /// `out` must be at least [`VENDOR_FIXED_SIZE`] + 32 bytes.
    fn encode(&self, out: &mut [u8]) -> usize {
        let vslen = string_len(&self.vendor);
        let pslen = string_len(&self.product);
        out[0..16].copy_from_slice(&self.uuid);
        out[16..18].copy_from_slice(&self.product_id.to_le_bytes());
        out[18..20].copy_from_slice(&self.product_ver.to_le_bytes());
        out[20] = vslen as u8;
        out[21] = pslen as u8;
        out[22..22 + vslen].copy_from_slice(&self.vendor[..vslen]);
        out[22 + vslen..22 + vslen + pslen].copy_from_slice(&self.product[..pslen]);
        VENDOR_FIXED_SIZE + vslen + pslen
    }

    /// On-the-wire size of this atom's data.
    fn data_len(&self) -> usize {
        VENDOR_FIXED_SIZE + string_len(&self.vendor) + string_len(&self.product)
    }

    /// Parses a vendor-info atom body.
    fn decode(data: &[u8]) -> Option<Self> {
        if data.len() < VENDOR_FIXED_SIZE {
            return None;
        }
        let mut uuid = [0u8; 16];
        uuid.copy_from_slice(&data[0..16]);
        let product_id = u16::from_le_bytes([data[16], data[17]]);
        let product_ver = u16::from_le_bytes([data[18], data[19]]);
        let vslen = data[20] as usize;
        let pslen = data[21] as usize;
        let mut vendor = [0u8; 16];
        let mut product = [0u8; 16];
        let v_start = VENDOR_FIXED_SIZE;
        let v_end = v_start + vslen;
        let p_end = v_end + pslen;
        if data.len() < p_end {
            return None;
        }
        let vcopy = vslen.min(16);
        let pcopy = pslen.min(16);
        vendor[..vcopy].copy_from_slice(&data[v_start..v_start + vcopy]);
        product[..pcopy].copy_from_slice(&data[v_end..v_end + pcopy]);
        Some(VendorInfoAtom {
            uuid,
            product_id,
            product_ver,
            vendor,
            product,
        })
    }
}

impl GpioMapAtom {
    /// Encodes a bank0 GPIO map body (30 bytes) into `out`.
    fn encode_bank0(&self, out: &mut [u8; 2 + GPIO_COUNT]) {
        out[0] = self.flags;
        out[1] = self.power;
        out[2..].copy_from_slice(&self.pins);
    }

    /// Encodes a bank1 GPIO map body (20 bytes) into `out`.
    fn encode_bank1(&self, out: &mut [u8; 2 + GPIO_COUNT_BANK1]) {
        out[0] = self.flags;
        out[1] = self.power;
        out[2..].copy_from_slice(&self.pins[..GPIO_COUNT_BANK1]);
    }

    /// Parses a GPIO map body; `pins` beyond the supplied data are left zero.
    fn decode(data: &[u8]) -> Option<Self> {
        if data.len() < 2 {
            return None;
        }
        let mut pins = [0u8; GPIO_COUNT];
        let n = (data.len() - 2).min(GPIO_COUNT);
        pins[..n].copy_from_slice(&data[2..2 + n]);
        Some(GpioMapAtom {
            flags: data[0],
            power: data[1],
            pins,
        })
    }
}

impl Eeprom {
    /// Parses an EEPROM image from a byte slice, validating the signature.
    ///
    /// Per-atom CRC-16 values are **not** verified here (use [`Eeprom::verify`]
    /// for that). This variant allocates owned buffers for atoms.
    #[cfg(feature = "alloc")]
    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < HEADER_SIZE {
            return Err("Not enough data for EEPROM header");
        }
        if data[0..4] != EEPROM_SIGNATURE {
            return Err("Invalid EEPROM signature");
        }
        let numatoms = u16::from_le_bytes([data[6], data[7]]);
        let eeplen = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let header = EepromHeader {
            signature: EEPROM_SIGNATURE,
            version: data[4],
            reserved: data[5],
            numatoms,
            eeplen,
        };

        let mut offset = HEADER_SIZE;
        let mut vendor_info = None;
        let mut gpio_map_bank0 = None;
        let mut dt_blob = None;
        let mut gpio_map_bank1 = None;
        let mut power_supply = None;
        let mut custom_atoms = Vec::new();

        for _ in 0..numatoms {
            if data.len() < offset + ATOM_HDR_SIZE {
                return Err("Not enough data for AtomHeader");
            }
            let atom_type = u16::from_le_bytes([data[offset], data[offset + 1]]);
            let dlen = u32::from_le_bytes([
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]) as usize;
            if dlen < CRC_SIZE {
                return Err("Atom dlen smaller than CRC");
            }
            let data_len = dlen - CRC_SIZE;
            let body_start = offset + ATOM_HDR_SIZE;
            if data.len() < body_start + data_len + CRC_SIZE {
                return Err("Not enough data for atom");
            }
            let body = &data[body_start..body_start + data_len];
            match atom_type {
                x if x == AtomType::VendorInfo as u16 => {
                    vendor_info = VendorInfoAtom::decode(body);
                }
                x if x == AtomType::GpioMapBank0 as u16 => {
                    gpio_map_bank0 = GpioMapAtom::decode(body);
                }
                x if x == AtomType::DtBlob as u16 => {
                    if !body.is_empty() {
                        dt_blob = Some(body.to_vec());
                    }
                }
                x if x == AtomType::GpioMapBank1 as u16 => {
                    gpio_map_bank1 = GpioMapAtom::decode(body);
                }
                x if x == AtomType::PowerSupply as u16 => {
                    if body.len() >= 4 {
                        power_supply =
                            Some(u32::from_le_bytes([body[0], body[1], body[2], body[3]]));
                    }
                }
                other => {
                    custom_atoms.push((other as u8, body.to_vec()));
                }
            }
            offset = body_start + data_len + CRC_SIZE;
        }
        Ok(Eeprom {
            header,
            vendor_info: vendor_info.ok_or("VendorInfo atom not found")?,
            gpio_map_bank0: gpio_map_bank0.ok_or("GpioMapBank0 atom not found")?,
            dt_blob,
            gpio_map_bank1,
            power_supply,
            custom_atoms,
        })
    }

    /// Parses an EEPROM image from a `'static` byte slice without heap
    /// allocations (`no_std`). Custom atoms are skipped in this mode.
    #[cfg(not(feature = "alloc"))]
    pub fn from_bytes_no_alloc(data: &'static [u8]) -> Result<Self, &'static str> {
        if data.len() < HEADER_SIZE {
            return Err("Not enough data for EEPROM header");
        }
        if data[0..4] != EEPROM_SIGNATURE {
            return Err("Invalid EEPROM signature");
        }
        let numatoms = u16::from_le_bytes([data[6], data[7]]);
        let eeplen = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let header = EepromHeader {
            signature: EEPROM_SIGNATURE,
            version: data[4],
            reserved: data[5],
            numatoms,
            eeplen,
        };

        let mut offset = HEADER_SIZE;
        let mut vendor_info = None;
        let mut gpio_map_bank0 = None;
        let mut dt_blob = None;
        let mut gpio_map_bank1 = None;
        let mut power_supply = None;
        let custom_atoms: &'static [(u8, &'static [u8])] = &[];

        for _ in 0..numatoms {
            if data.len() < offset + ATOM_HDR_SIZE {
                return Err("Not enough data for AtomHeader");
            }
            let atom_type = u16::from_le_bytes([data[offset], data[offset + 1]]);
            let dlen = u32::from_le_bytes([
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]) as usize;
            if dlen < CRC_SIZE {
                return Err("Atom dlen smaller than CRC");
            }
            let data_len = dlen - CRC_SIZE;
            let body_start = offset + ATOM_HDR_SIZE;
            if data.len() < body_start + data_len + CRC_SIZE {
                return Err("Not enough data for atom");
            }
            let body = &data[body_start..body_start + data_len];
            match atom_type {
                x if x == AtomType::VendorInfo as u16 => {
                    vendor_info = VendorInfoAtom::decode(body);
                }
                x if x == AtomType::GpioMapBank0 as u16 => {
                    gpio_map_bank0 = GpioMapAtom::decode(body);
                }
                x if x == AtomType::DtBlob as u16 => {
                    if !body.is_empty() {
                        dt_blob = Some(body);
                    }
                }
                x if x == AtomType::GpioMapBank1 as u16 => {
                    gpio_map_bank1 = GpioMapAtom::decode(body);
                }
                x if x == AtomType::PowerSupply as u16 => {
                    if body.len() >= 4 {
                        power_supply =
                            Some(u32::from_le_bytes([body[0], body[1], body[2], body[3]]));
                    }
                }
                _ => {}
            }
            offset = body_start + data_len + CRC_SIZE;
        }
        Ok(Eeprom {
            header,
            vendor_info: vendor_info.ok_or("VendorInfo atom not found")?,
            gpio_map_bank0: gpio_map_bank0.ok_or("GpioMapBank0 atom not found")?,
            dt_blob,
            gpio_map_bank1,
            power_supply,
            custom_atoms,
        })
    }

    /// Verifies the signature and every per-atom CRC-16 of a serialized image.
    ///
    /// Returns `true` only when the header signature is valid and each atom's
    /// stored CRC matches the CRC-16 recomputed over its header and data.
    pub fn verify(data: &[u8]) -> bool {
        if data.len() < HEADER_SIZE || data[0..4] != EEPROM_SIGNATURE {
            return false;
        }
        let numatoms = u16::from_le_bytes([data[6], data[7]]);
        let mut offset = HEADER_SIZE;
        for _ in 0..numatoms {
            if data.len() < offset + ATOM_HDR_SIZE {
                return false;
            }
            let dlen = u32::from_le_bytes([
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]) as usize;
            if dlen < CRC_SIZE {
                return false;
            }
            let data_len = dlen - CRC_SIZE;
            let crc_off = offset + ATOM_HDR_SIZE + data_len;
            if data.len() < crc_off + CRC_SIZE {
                return false;
            }
            let expected = crc16(&data[offset..crc_off]);
            let stored = u16::from_le_bytes([data[crc_off], data[crc_off + 1]]);
            if expected != stored {
                return false;
            }
            offset = crc_off + CRC_SIZE;
        }
        true
    }

    /// Checks if EEPROM contains valid data (by signature and version)
    pub fn is_valid(&self) -> bool {
        self.header.signature == EEPROM_SIGNATURE && self.header.version != 0
    }

    pub fn add_vendor_info(&mut self, atom: VendorInfoAtom) {
        self.vendor_info = atom;
        self.update_header();
    }
    pub fn add_gpio_map_bank0(&mut self, atom: GpioMapAtom) {
        self.gpio_map_bank0 = atom;
        self.update_header();
    }

    #[cfg(feature = "alloc")]
    pub fn add_dt_blob(&mut self, blob: Vec<u8>) {
        self.dt_blob = Some(blob);
        self.update_header();
    }

    #[cfg(not(feature = "alloc"))]
    pub fn add_dt_blob_static(&mut self, blob: &'static [u8]) {
        self.dt_blob = Some(blob);
        self.update_header();
    }

    pub fn add_gpio_map_bank1(&mut self, atom: GpioMapAtom) {
        self.gpio_map_bank1 = Some(atom);
        self.update_header();
    }

    /// Sets the power-supply atom (`0x0006`): required back-power current in mA.
    pub fn add_power_supply(&mut self, current_ma: u32) {
        self.power_supply = Some(current_ma);
        self.update_header();
    }

    #[cfg(feature = "alloc")]
    pub fn add_custom_atom(&mut self, atom_type: u8, data: Vec<u8>) {
        self.custom_atoms.push((atom_type, data));
        self.update_header();
    }

    #[cfg(not(feature = "alloc"))]
    pub fn set_custom_atoms(&mut self, atoms: &'static [(u8, &'static [u8])]) {
        self.custom_atoms = atoms;
        self.update_header();
    }

    /// Number of atoms this EEPROM will serialize to.
    pub fn atom_count(&self) -> u16 {
        let mut n: u16 = 2; // VendorInfo + GPIO bank0 are always present
        if self.dt_blob.is_some() {
            n += 1;
        }
        if self.gpio_map_bank1.is_some() {
            n += 1;
        }
        if self.power_supply.is_some() {
            n += 1;
        }
        n += self.custom_atoms.len() as u16;
        n
    }

    /// Recalculate `numatoms` and `eeplen` in the header.
    pub fn update_header(&mut self) {
        self.header.signature = EEPROM_SIGNATURE;
        self.header.numatoms = self.atom_count();
        self.header.eeplen = self.calculate_serialized_size() as u32;
    }

    /// Total serialized size in bytes, including the header and every atom's
    /// header and CRC.
    pub fn calculate_serialized_size(&self) -> usize {
        let atom_overhead = ATOM_HDR_SIZE + CRC_SIZE;
        let mut size = HEADER_SIZE;
        size += atom_overhead + self.vendor_info.data_len();
        size += atom_overhead + (2 + GPIO_COUNT);

        if let Some(ref blob) = self.dt_blob {
            size += atom_overhead + blob.len();
        }
        if self.gpio_map_bank1.is_some() {
            size += atom_overhead + (2 + GPIO_COUNT_BANK1);
        }
        if self.power_supply.is_some() {
            size += atom_overhead + 4;
        }

        #[cfg(feature = "alloc")]
        for (_tag, data) in &self.custom_atoms {
            size += atom_overhead + data.len();
        }
        #[cfg(not(feature = "alloc"))]
        for (_tag, data) in self.custom_atoms {
            size += atom_overhead + data.len();
        }

        size
    }

    /// Serializes the complete, spec-compliant HAT EEPROM image into `buf`.
    ///
    /// Writes the 12-byte header followed by every atom with its 8-byte header,
    /// data, and trailing CRC-16. Works without heap allocation. Returns the
    /// number of bytes written, or [`EhatromError::BufferTooSmall`].
    pub fn serialize_into(&self, buf: &mut [u8]) -> Result<usize, EhatromError> {
        let total = self.calculate_serialized_size();
        if buf.len() < total {
            return Err(EhatromError::BufferTooSmall);
        }

        // Header.
        buf[0..4].copy_from_slice(&EEPROM_SIGNATURE);
        buf[4] = if self.header.version == 0 {
            FORMAT_VERSION
        } else {
            self.header.version
        };
        buf[5] = self.header.reserved;
        buf[6..8].copy_from_slice(&self.atom_count().to_le_bytes());
        buf[8..12].copy_from_slice(&(total as u32).to_le_bytes());

        let mut offset = HEADER_SIZE;
        let mut count: u16 = 0;

        // Vendor info.
        let mut vbuf = [0u8; VENDOR_FIXED_SIZE + 32];
        let vlen = self.vendor_info.encode(&mut vbuf);
        write_atom(
            buf,
            &mut offset,
            &mut count,
            AtomType::VendorInfo as u16,
            &vbuf[..vlen],
        )?;

        // GPIO bank0.
        let mut gbuf = [0u8; 2 + GPIO_COUNT];
        self.gpio_map_bank0.encode_bank0(&mut gbuf);
        write_atom(
            buf,
            &mut offset,
            &mut count,
            AtomType::GpioMapBank0 as u16,
            &gbuf,
        )?;

        // Device-tree blob.
        if let Some(ref blob) = self.dt_blob {
            write_atom(buf, &mut offset, &mut count, AtomType::DtBlob as u16, blob)?;
        }

        // GPIO bank1.
        if let Some(ref bank1) = self.gpio_map_bank1 {
            let mut g1 = [0u8; 2 + GPIO_COUNT_BANK1];
            bank1.encode_bank1(&mut g1);
            write_atom(
                buf,
                &mut offset,
                &mut count,
                AtomType::GpioMapBank1 as u16,
                &g1,
            )?;
        }

        // Power supply.
        if let Some(current_ma) = self.power_supply {
            write_atom(
                buf,
                &mut offset,
                &mut count,
                AtomType::PowerSupply as u16,
                &current_ma.to_le_bytes(),
            )?;
        }

        // Custom atoms (always emitted with the spec custom type 0x0004).
        #[cfg(feature = "alloc")]
        for (_tag, data) in &self.custom_atoms {
            write_atom(buf, &mut offset, &mut count, AtomType::Custom as u16, data)?;
        }
        #[cfg(not(feature = "alloc"))]
        for (_tag, data) in self.custom_atoms {
            write_atom(buf, &mut offset, &mut count, AtomType::Custom as u16, data)?;
        }

        Ok(offset)
    }

    /// Serializes the EEPROM to a freshly allocated `Vec<u8>`.
    #[cfg(feature = "alloc")]
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = alloc::vec![0u8; self.calculate_serialized_size()];
        // Size is exact, so serialization cannot fail.
        let len = self
            .serialize_into(&mut bytes)
            .expect("buffer sized by calculate_serialized_size");
        bytes.truncate(len);
        bytes
    }

    /// Serializes the EEPROM into a caller-provided buffer (`no_std`).
    #[cfg(not(feature = "alloc"))]
    pub fn serialize_to_slice(&self, buffer: &mut [u8]) -> Result<usize, EhatromError> {
        self.serialize_into(buffer)
    }

    pub fn set_version(&mut self, version: u8) {
        self.header.version = version;
    }
}

/// Writes one atom (header + data + CRC-16) into `buf` at `*offset`.
///
/// `dlen` is set to `data.len() + 2` and the CRC-16 is computed over the 8-byte
/// atom header together with `data`, exactly as the reference `eepmake` does.
fn write_atom(
    buf: &mut [u8],
    offset: &mut usize,
    count: &mut u16,
    atom_type: u16,
    data: &[u8],
) -> Result<(), EhatromError> {
    let start = *offset;
    let end = start + ATOM_HDR_SIZE + data.len() + CRC_SIZE;
    if end > buf.len() {
        return Err(EhatromError::BufferTooSmall);
    }
    let dlen = (data.len() + CRC_SIZE) as u32;
    buf[start..start + 2].copy_from_slice(&atom_type.to_le_bytes());
    buf[start + 2..start + 4].copy_from_slice(&count.to_le_bytes());
    buf[start + 4..start + 8].copy_from_slice(&dlen.to_le_bytes());
    let body_end = start + ATOM_HDR_SIZE + data.len();
    buf[start + ATOM_HDR_SIZE..body_end].copy_from_slice(data);
    let crc = crc16(&buf[start..body_end]);
    buf[body_end..body_end + CRC_SIZE].copy_from_slice(&crc.to_le_bytes());
    *offset = body_end + CRC_SIZE;
    *count += 1;
    Ok(())
}

impl From<u16> for AtomType {
    fn from(val: u16) -> Self {
        match val {
            0x0001 => AtomType::VendorInfo,
            0x0002 => AtomType::GpioMapBank0,
            0x0003 => AtomType::DtBlob,
            0x0004 => AtomType::Custom,
            0x0005 => AtomType::GpioMapBank1,
            0x0006 => AtomType::PowerSupply,
            _ => AtomType::Unknown,
        }
    }
}

/// Writes a prepared EEPROM image to the target I2C device.
///
/// The function performs basic structural validation (signature, header fields,
/// and every per-atom CRC-16) before issuing page writes to the given address.
#[cfg(all(feature = "linux", any(target_os = "linux", target_os = "android")))]
pub fn write_to_eeprom_i2c(data: &[u8], dev_path: &str, addr: u16) -> Result<(), EhatromError> {
    // Validate EEPROM data before writing
    if data.len() < HEADER_SIZE {
        return Err(EhatromError::InvalidData);
    }

    // Check signature "R-Pi"
    if data[0..4] != EEPROM_SIGNATURE {
        return Err(EhatromError::InvalidData);
    }

    // Verify that the file contains a proper EEPROM structure
    let version = data[4];
    if version == 0 {
        return Err(EhatromError::InvalidData);
    }
    let numatoms = u16::from_le_bytes([data[6], data[7]]);
    let eeplen = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
    if numatoms == 0 {
        return Err(EhatromError::InvalidData);
    }
    if eeplen as usize > data.len() {
        return Err(EhatromError::InvalidData);
    }
    if (eeplen as usize) < HEADER_SIZE {
        return Err(EhatromError::InvalidData);
    }
    // Verify every per-atom CRC-16.
    if !Eeprom::verify(data) {
        return Err(EhatromError::InvalidCrc);
    }

    let mut dev = LinuxI2CDevice::new(dev_path, addr).map_err(|_| EhatromError::I2cError)?;
    // EEPROM HAT: use page write (16 bytes per page) with 2-byte offset
    let page_size = 16;
    let mut offset = 0u16;
    while (offset as usize) < data.len() {
        let end = (offset as usize + page_size).min(data.len());
        #[cfg(feature = "alloc")]
        {
            let mut buf = Vec::with_capacity(2 + page_size);
            buf.push((offset >> 8) as u8);
            buf.push((offset & 0xFF) as u8);
            buf.extend_from_slice(&data[offset as usize..end]);
            dev.write(&buf).map_err(|_| EhatromError::I2cError)?;
        }
        #[cfg(not(feature = "alloc"))]
        {
            // For no_std, use fixed-size buffer
            let mut buf = [0u8; 18]; // 2 bytes offset + 16 bytes data max
            buf[0] = (offset >> 8) as u8;
            buf[1] = (offset & 0xFF) as u8;
            let data_len = end - offset as usize;
            buf[2..2 + data_len].copy_from_slice(&data[offset as usize..end]);
            dev.write(&buf[..2 + data_len])
                .map_err(|_| EhatromError::I2cError)?;
        }

        // Sleep replacement for no_std
        #[cfg(feature = "std")]
        std::thread::sleep(std::time::Duration::from_millis(10));
        #[cfg(not(feature = "std"))]
        {
            // For bare-metal, implement busy-wait delay
            // This is platform-specific and should be replaced with proper delay
            for _ in 0..100000 {
                core::hint::spin_loop();
            }
        }

        offset += (end - offset as usize) as u16;
    }
    Ok(())
}

/// Reads EEPROM contents from the target I2C device into the provided buffer.
///
/// Reading starts at the specified offset and continues until the buffer is
/// filled or the device reports an error.
#[cfg(all(feature = "linux", any(target_os = "linux", target_os = "android")))]
pub fn read_from_eeprom_i2c(
    buf: &mut [u8],
    dev_path: &str,
    addr: u16,
    offset: u16,
) -> Result<(), EhatromError> {
    let mut dev = LinuxI2CDevice::new(dev_path, addr).map_err(|_| EhatromError::I2cError)?;
    const PAGE_SIZE: usize = 32; // Safe default size for most EEPROM chips
    let mut total_read = 0;
    let mut current_offset = offset;
    while total_read < buf.len() {
        let chunk_size = PAGE_SIZE.min(buf.len() - total_read);
        let offset_bytes = [(current_offset >> 8) as u8, (current_offset & 0xFF) as u8];
        dev.write(&offset_bytes)
            .map_err(|_| EhatromError::I2cError)?;
        let chunk_buf = &mut buf[total_read..total_read + chunk_size];
        dev.read(chunk_buf).map_err(|_| EhatromError::I2cError)?;
        total_read += chunk_size;
        current_offset += chunk_size as u16;
    }
    Ok(())
}

#[cfg(all(feature = "linux", any(target_os = "linux", target_os = "android")))]
pub mod detect;
#[cfg(all(feature = "linux", any(target_os = "linux", target_os = "android")))]
pub use detect::{detect_all_i2c_devices, detect_and_show_eeprom_info, find_i2c_devices};

impl VendorInfoAtom {
    /// Creates a `VendorInfoAtom` from strings.
    ///
    /// The vendor and product strings are stored in 16-byte buffers (truncated
    /// if longer); on serialization only the used prefix is written, with the
    /// spec `vslen`/`pslen` length fields set accordingly.
    pub fn new(
        product_id: u16,
        product_ver: u16,
        vendor: &str,
        product: &str,
        uuid: [u8; 16],
    ) -> Self {
        let mut vendor_arr = [0u8; 16];
        let mut product_arr = [0u8; 16];
        let vendor_bytes = vendor.as_bytes();
        let product_bytes = product.as_bytes();
        let vendor_len = vendor_bytes.len().min(16);
        let product_len = product_bytes.len().min(16);
        vendor_arr[..vendor_len].copy_from_slice(&vendor_bytes[..vendor_len]);
        product_arr[..product_len].copy_from_slice(&product_bytes[..product_len]);
        VendorInfoAtom {
            uuid,
            product_id,
            product_ver,
            vendor: vendor_arr,
            product: product_arr,
        }
    }
}

impl core::fmt::Display for Eeprom {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "EEPROM Header:\n{}", self.header)?;
        writeln!(f, "\nVendor Info:\n{}", self.vendor_info)?;
        writeln!(f, "\nGPIO Map Bank0:\n{}", self.gpio_map_bank0)?;
        if let Some(ref dt_blob) = self.dt_blob {
            writeln!(f, "\nDT Blob: {} bytes", dt_blob.len())?;
        }
        if let Some(ref bank1) = self.gpio_map_bank1 {
            writeln!(f, "\nGPIO Map Bank1:\n{bank1}")?
        }
        if let Some(current_ma) = self.power_supply {
            writeln!(f, "\nPower Supply: {current_ma} mA")?
        }
        #[cfg(feature = "alloc")]
        if !self.custom_atoms.is_empty() {
            writeln!(f, "\nCustom Atoms:")?;
            for (typ, data) in &self.custom_atoms {
                writeln!(f, "  type: 0x{typ:02X}, data: {data:02X?}")?
            }
        }
        #[cfg(not(feature = "alloc"))]
        if !self.custom_atoms.is_empty() {
            writeln!(f, "\nCustom Atoms:")?;
            for (typ, data) in self.custom_atoms {
                writeln!(f, "  type: 0x{typ:02X}, data: {data:02X?}")?
            }
        }
        Ok(())
    }
}
