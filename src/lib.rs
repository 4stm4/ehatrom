//  _  _       _             _  _
// | || |  ___| |_ _ __ ___ | || |
// | || |_/ __| __| '_ ` _ \| || |_
// |__   _\__ | |_| | | | | |__   _|
//   |_| |___/\__|_|_|_| |_|  |_|
//! # ehatrom — EEPROM HAT library for Raspberry Pi HATs
//! - [Documentation (docs.rs)](https://docs.rs/ehatrom)
//! - [GitHub](https://github.com/4stm4/ehatrom)

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
    /// Invalid CRC32 checksum
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
            EhatromError::InvalidCrc => write!(f, "Invalid CRC32 checksum"),
            EhatromError::BufferTooSmall => write!(f, "Buffer too small for operation"),
            EhatromError::DeviceNotFound => write!(f, "Device not found"),
            EhatromError::Timeout => write!(f, "Timeout during operation"),
        }
    }
}

// Implement Error trait when std is available
#[cfg(feature = "std")]
impl std::error::Error for EhatromError {}

pub mod utils;
use utils::crc32::Hasher;

#[cfg(all(feature = "linux", any(target_os = "linux", target_os = "android")))]
use i2cdev::{core::I2CDevice, linux::LinuxI2CDevice};

/// EEPROM header structure for Raspberry Pi
/// The header is always 12 bytes long and follows the packed representation.
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
    /// Total length of EEPROM data (Little Endian)
    pub eeplen: u32,
}

impl EepromHeader {
    /// Creates a new EepromHeader with default values
    pub const fn new() -> Self {
        EepromHeader {
            signature: *b"R-Pi",
            version: 1,
            reserved: 0,
            numatoms: 0,
            eeplen: 0,
        }
    }
}

/// Main structure representing EEPROM atom header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct AtomHeader {
    /// Type identifier of the atom (e.g. 0xD0 for vendor info)
    pub atom_type: u8,
    /// Number of structures in this atom (typically 1)
    pub count: u8,
    /// Length of atom data in bytes (Little Endian)
    pub dlen: u16,
    /// Reserved field (must be 0)
    pub reserved: u32,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomType {
    VendorInfo = 0x01,
    GpioMapBank0 = 0x02,
    DtBlob = 0x03,
    GpioMapBank1 = 0x04,
    Unknown,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct VendorInfoAtom {
    pub vendor_id: u16,    // Vendor ID
    pub product_id: u16,   // Product ID
    pub product_ver: u16,  // Product version
    pub vendor: [u8; 16],  // Vendor name (null-terminated)
    pub product: [u8; 16], // Product name (null-terminated)
    pub uuid: [u8; 16],    // UUID
}

impl fmt::Debug for VendorInfoAtom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(feature = "alloc")]
        {
            let vendor_str = String::from_utf8_lossy(&self.vendor)
                .trim_end_matches('\0')
                .to_string();
            let product_str = String::from_utf8_lossy(&self.product)
                .trim_end_matches('\0')
                .to_string();
            let vendor_id = self.vendor_id;
            let product_id = self.product_id;
            let product_ver = self.product_ver;
            let uuid = self.uuid;
            write!(
                f,
                "VendorInfoAtom {{ vendor_id: {vendor_id}, product_id: {product_id}, product_ver: {product_ver}, vendor: \"{vendor_str}\", product: \"{product_str}\", uuid: {uuid:?} }}"
            )
        }
        #[cfg(not(feature = "alloc"))]
        {
            // In no_std environment, display raw bytes
            let vendor_id = self.vendor_id;
            let product_id = self.product_id;
            let product_ver = self.product_ver;
            let vendor = self.vendor;
            let product = self.product;
            let uuid = self.uuid;
            write!(
                f,
                "VendorInfoAtom {{ vendor_id: {vendor_id}, product_id: {product_id}, product_ver: {product_ver}, vendor: {vendor:?}, product: {product:?}, uuid: {uuid:?} }}"
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
        let reserved = self.reserved;
        write!(
            f,
            "atom_type: 0x{atom_type:02X}\ncount: {count}\ndlen: {dlen}\nreserved: {reserved}",
        )
    }
}

impl core::fmt::Display for VendorInfoAtom {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let vendor_id = self.vendor_id;
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
                "vendor_id: 0x{vendor_id:04X}\nproduct_id: 0x{product_id:04X}\nproduct_ver: {product_ver}\nvendor: {vendor}\nproduct: {product}\nuuid: {uuid:02X?}"
            )
        }
        #[cfg(not(feature = "alloc"))]
        {
            // For no_std, find null terminator manually
            let vendor_len = vendor_buf
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(vendor_buf.len());
            let product_len = product_buf
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(product_buf.len());

            // Try to display as UTF-8 strings, fallback to hex
            match (
                core::str::from_utf8(&vendor_buf[..vendor_len]),
                core::str::from_utf8(&product_buf[..product_len]),
            ) {
                (Ok(vendor), Ok(product)) => {
                    write!(
                        f,
                        "vendor_id: 0x{vendor_id:04X}\nproduct_id: 0x{product_id:04X}\nproduct_ver: {product_ver}\nvendor: {vendor}\nproduct: {product}\nuuid: {uuid:02X?}"
                    )
                }
                _ => {
                    write!(
                        f,
                        "vendor_id: 0x{vendor_id:04X}\nproduct_id: 0x{product_id:04X}\nproduct_ver: {product_ver}\nvendor: {vendor_buf:02X?}\nproduct: {product_buf:02X?}\nuuid: {uuid:02X?}"
                    )
                }
            }
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct GpioMapAtom {
    pub flags: u16,
    pub pins: [u8; 28],
}

impl core::fmt::Display for GpioMapAtom {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let flags = self.flags;
        let pins = self.pins;
        write!(f, "flags: 0x{flags:04X}\npins: {pins:?}")
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
    #[cfg(feature = "alloc")]
    pub custom_atoms: Vec<(u8, Vec<u8>)>, // (atom_type, data)
    #[cfg(not(feature = "alloc"))]
    pub custom_atoms: &'static [(u8, &'static [u8])], // Static data for no_std
}

impl Eeprom {
    /// Reads EEPROM structure from a byte slice
    #[cfg(feature = "alloc")]
    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        use core::mem::size_of;
        use core::ptr::read_unaligned;

        if data.len() < size_of::<EepromHeader>() {
            return Err("Not enough data for EEPROM header");
        }
        // Read header
        let header = unsafe { read_unaligned(data.as_ptr() as *const EepromHeader) };
        if &header.signature != b"R-Pi" {
            return Err("Invalid EEPROM signature");
        }
        let mut offset = size_of::<EepromHeader>();
        let mut vendor_info = None;
        let mut gpio_map_bank0 = None;
        let mut dt_blob = None;
        let mut gpio_map_bank1 = None;
        let mut custom_atoms = Vec::new();
        for _ in 0..header.numatoms {
            if data.len() < offset + size_of::<AtomHeader>() {
                return Err("Not enough data for AtomHeader");
            }
            let atom_header =
                unsafe { read_unaligned(data[offset..].as_ptr() as *const AtomHeader) };
            offset += size_of::<AtomHeader>();
            if data.len() < offset + atom_header.dlen as usize {
                return Err("Not enough data for atom");
            }
            match AtomType::from(atom_header.atom_type) {
                AtomType::VendorInfo => {
                    if atom_header.dlen as usize >= size_of::<VendorInfoAtom>() {
                        vendor_info = Some(unsafe {
                            read_unaligned(data[offset..].as_ptr() as *const VendorInfoAtom)
                        });
                    }
                }
                AtomType::GpioMapBank0 => {
                    if atom_header.dlen as usize >= size_of::<GpioMapAtom>() {
                        gpio_map_bank0 = Some(unsafe {
                            read_unaligned(data[offset..].as_ptr() as *const GpioMapAtom)
                        });
                    }
                }
                AtomType::DtBlob => {
                    let dlen = atom_header.dlen as usize;
                    if dlen > 0 && data.len() >= offset + dlen {
                        dt_blob = Some(data[offset..offset + dlen].to_vec());
                    }
                }
                AtomType::GpioMapBank1 => {
                    if atom_header.dlen as usize >= size_of::<GpioMapAtom>() {
                        gpio_map_bank1 = Some(unsafe {
                            read_unaligned(data[offset..].as_ptr() as *const GpioMapAtom)
                        });
                    }
                }
                AtomType::Unknown => {
                    // Save custom atom (type and data)
                    let dlen = atom_header.dlen as usize;
                    if dlen > 0 && data.len() >= offset + dlen {
                        custom_atoms
                            .push((atom_header.atom_type, data[offset..offset + dlen].to_vec()));
                    }
                }
            }
            offset += atom_header.dlen as usize;
        }
        Ok(Eeprom {
            header,
            vendor_info: vendor_info.ok_or("VendorInfo atom not found")?,
            gpio_map_bank0: gpio_map_bank0.ok_or("GpioMapBank0 atom not found")?,
            dt_blob,
            gpio_map_bank1,
            custom_atoms,
        })
    }

    /// Reads EEPROM structure from a byte slice (no_std version)
    /// Returns references to data instead of owned data
    #[cfg(not(feature = "alloc"))]
    pub fn from_bytes_no_alloc(data: &'static [u8]) -> Result<Self, &'static str> {
        use core::mem::size_of;
        use core::ptr::read_unaligned;

        if data.len() < size_of::<EepromHeader>() {
            return Err("Not enough data for EEPROM header");
        }
        // Read header
        let header = unsafe { read_unaligned(data.as_ptr() as *const EepromHeader) };
        if &header.signature != b"R-Pi" {
            return Err("Invalid EEPROM signature");
        }
        let mut offset = size_of::<EepromHeader>();
        let mut vendor_info = None;
        let mut gpio_map_bank0 = None;
        let mut dt_blob = None;
        let mut gpio_map_bank1 = None;
        let custom_atoms: &'static [(u8, &'static [u8])] = &[];

        for _ in 0..header.numatoms {
            if data.len() < offset + size_of::<AtomHeader>() {
                return Err("Not enough data for AtomHeader");
            }
            let atom_header =
                unsafe { read_unaligned(data[offset..].as_ptr() as *const AtomHeader) };
            offset += size_of::<AtomHeader>();
            if data.len() < offset + atom_header.dlen as usize {
                return Err("Not enough data for atom");
            }
            match AtomType::from(atom_header.atom_type) {
                AtomType::VendorInfo => {
                    if atom_header.dlen as usize >= size_of::<VendorInfoAtom>() {
                        vendor_info = Some(unsafe {
                            read_unaligned(data[offset..].as_ptr() as *const VendorInfoAtom)
                        });
                    }
                }
                AtomType::GpioMapBank0 => {
                    if atom_header.dlen as usize >= size_of::<GpioMapAtom>() {
                        gpio_map_bank0 = Some(unsafe {
                            read_unaligned(data[offset..].as_ptr() as *const GpioMapAtom)
                        });
                    }
                }
                AtomType::DtBlob => {
                    let dlen = atom_header.dlen as usize;
                    if dlen > 0 && data.len() >= offset + dlen {
                        dt_blob = Some(&data[offset..offset + dlen]);
                    }
                }
                AtomType::GpioMapBank1 => {
                    if atom_header.dlen as usize >= size_of::<GpioMapAtom>() {
                        gpio_map_bank1 = Some(unsafe {
                            read_unaligned(data[offset..].as_ptr() as *const GpioMapAtom)
                        });
                    }
                }
                AtomType::Unknown => {
                    // For no_std, we'll skip custom atoms for now
                    // In real implementation, you'd need a pre-allocated array or similar
                }
            }
            offset += atom_header.dlen as usize;
        }
        Ok(Eeprom {
            header,
            vendor_info: vendor_info.ok_or("VendorInfo atom not found")?,
            gpio_map_bank0: gpio_map_bank0.ok_or("GpioMapBank0 atom not found")?,
            dt_blob,
            gpio_map_bank1,
            custom_atoms,
        })
    }

    /// Checks if EEPROM contains valid data (by signature and version)
    pub fn is_valid(&self) -> bool {
        self.header.signature == *b"R-Pi" && self.header.version != 0
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
    /// Recalculate numatoms and eeplen after adding atoms
    pub fn update_header(&mut self) {
        let mut numatoms = 2; // VendorInfo and GPIO bank0 are always present
        let mut eeplen = core::mem::size_of::<EepromHeader>()
            + core::mem::size_of::<AtomHeader>() * 2
            + core::mem::size_of::<VendorInfoAtom>()
            + core::mem::size_of::<GpioMapAtom>();
        if self.dt_blob.is_some() {
            numatoms += 1;
            if let Some(ref blob) = self.dt_blob {
                eeplen += core::mem::size_of::<AtomHeader>() + blob.len();
            }
        }
        if self.gpio_map_bank1.is_some() {
            numatoms += 1;
            eeplen += core::mem::size_of::<AtomHeader>() + core::mem::size_of::<GpioMapAtom>();
        }
        #[cfg(feature = "alloc")]
        for (_atom_type, data) in &self.custom_atoms {
            numatoms += 1;
            eeplen += core::mem::size_of::<AtomHeader>() + data.len();
        }
        #[cfg(not(feature = "alloc"))]
        for (_atom_type, data) in self.custom_atoms {
            numatoms += 1;
            eeplen += core::mem::size_of::<AtomHeader>() + data.len();
        }
        self.header.numatoms = numatoms;
        self.header.eeplen = eeplen as u32;
    }

    /// Serialize with CRC32 appended (4 bytes LE)
    #[cfg(feature = "alloc")]
    pub fn serialize_with_crc(&self) -> Vec<u8> {
        let mut data = self.serialize();
        let mut hasher = Hasher::new();
        hasher.update(&data);
        let crc = hasher.finalize();
        data.extend_from_slice(&crc.to_le_bytes());
        data
    }

    /// Serialize with CRC32 to provided buffer (no_std version)
    #[cfg(not(feature = "alloc"))]
    pub fn serialize_with_crc_to_slice(&self, buffer: &mut [u8]) -> Result<usize, EhatromError> {
        let data_len = self.serialize_to_slice(buffer)?;
        if buffer.len() < data_len + 4 {
            return Err(EhatromError::BufferTooSmall);
        }

        let mut hasher = Hasher::new();
        hasher.update(&buffer[..data_len]);
        let crc = hasher.finalize();
        buffer[data_len..data_len + 4].copy_from_slice(&crc.to_le_bytes());
        Ok(data_len + 4)
    }
    /// CRC32 check (expects last 4 bytes to be CRC32 LE)
    pub fn verify_crc(data: &[u8]) -> bool {
        if data.len() < 4 {
            return false;
        }
        let (content, crc_bytes) = data.split_at(data.len() - 4);
        let mut hasher = Hasher::new();
        hasher.update(content);
        let crc = hasher.finalize();
        crc_bytes == crc.to_le_bytes()
    }

    /// CRC32 check with detailed error report
    #[cfg(feature = "std")]
    pub fn verify_crc_with_details(data: &[u8]) -> Result<(), EhatromError> {
        if data.len() < 4 {
            return Err(EhatromError::InvalidData);
        }
        let (content, crc_bytes) = data.split_at(data.len() - 4);
        let mut hasher = Hasher::new();
        hasher.update(content);
        let crc = hasher.finalize();
        let expected_crc =
            u32::from_le_bytes([crc_bytes[0], crc_bytes[1], crc_bytes[2], crc_bytes[3]]);

        if crc_bytes == crc.to_le_bytes() {
            Ok(())
        } else {
            Err(EhatromError::InvalidCrc)
        }
    }

    /// Serialize EEPROM structure to `Vec<u8>` (without CRC)
    #[cfg(feature = "alloc")]
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        self.serialize_to_vec(&mut bytes);
        bytes
    }

    /// Serialize EEPROM structure to buffer (no_std version)
    #[cfg(not(feature = "alloc"))]
    pub fn serialize_to_slice(&self, buffer: &mut [u8]) -> Result<usize, EhatromError> {
        let mut offset = 0;

        // Calculate required size first
        let required_size = self.calculate_serialized_size();
        if buffer.len() < required_size {
            return Err(EhatromError::BufferTooSmall);
        }

        self.serialize_to_buffer(buffer, &mut offset)?;
        Ok(offset)
    }

    /// Serialize EEPROM structure to buffer (universal version)
    pub fn serialize_to_slice_universal(&self, buffer: &mut [u8]) -> Result<usize, EhatromError> {
        let mut offset = 0;

        // Calculate required size first
        let required_size = self.calculate_serialized_size();
        if buffer.len() < required_size {
            return Err(EhatromError::BufferTooSmall);
        }

        self.serialize_to_buffer(buffer, &mut offset)?;
        Ok(offset)
    }

    /// Helper method to calculate total serialized size
    pub fn calculate_serialized_size(&self) -> usize {
        let mut size = core::mem::size_of::<EepromHeader>();
        size += core::mem::size_of::<AtomHeader>() * 2; // VendorInfo + GPIO bank0
        size += core::mem::size_of::<VendorInfoAtom>();
        size += core::mem::size_of::<GpioMapAtom>();

        if let Some(ref blob) = self.dt_blob {
            size += core::mem::size_of::<AtomHeader>() + blob.len();
        }

        if self.gpio_map_bank1.is_some() {
            size += core::mem::size_of::<AtomHeader>() + core::mem::size_of::<GpioMapAtom>();
        }

        #[cfg(feature = "alloc")]
        for (_atom_type, data) in &self.custom_atoms {
            size += core::mem::size_of::<AtomHeader>() + data.len();
        }

        #[cfg(not(feature = "alloc"))]
        for (_atom_type, data) in self.custom_atoms {
            size += core::mem::size_of::<AtomHeader>() + data.len();
        }

        size
    }

    /// Internal method to serialize into Vec (alloc version)
    #[cfg(feature = "alloc")]
    fn serialize_to_vec(&self, bytes: &mut Vec<u8>) {
        // EEPROM header
        let header_ptr = &self.header as *const _ as *const u8;
        bytes.extend_from_slice(unsafe {
            core::slice::from_raw_parts(header_ptr, core::mem::size_of::<EepromHeader>())
        });

        // VendorInfo
        let atom_header = AtomHeader {
            atom_type: AtomType::VendorInfo as u8,
            count: 1,
            dlen: core::mem::size_of::<VendorInfoAtom>() as u16,
            reserved: 0,
        };
        let atom_ptr = &atom_header as *const _ as *const u8;
        bytes.extend_from_slice(unsafe {
            core::slice::from_raw_parts(atom_ptr, core::mem::size_of::<AtomHeader>())
        });
        let vendor_ptr = &self.vendor_info as *const _ as *const u8;
        bytes.extend_from_slice(unsafe {
            core::slice::from_raw_parts(vendor_ptr, core::mem::size_of::<VendorInfoAtom>())
        });

        // GPIO bank0
        let atom_header = AtomHeader {
            atom_type: AtomType::GpioMapBank0 as u8,
            count: 1,
            dlen: core::mem::size_of::<GpioMapAtom>() as u16,
            reserved: 0,
        };
        let atom_ptr = &atom_header as *const _ as *const u8;
        bytes.extend_from_slice(unsafe {
            core::slice::from_raw_parts(atom_ptr, core::mem::size_of::<AtomHeader>())
        });
        let gpio_ptr = &self.gpio_map_bank0 as *const _ as *const u8;
        bytes.extend_from_slice(unsafe {
            core::slice::from_raw_parts(gpio_ptr, core::mem::size_of::<GpioMapAtom>())
        });

        // DT blob
        if let Some(ref blob) = self.dt_blob {
            let atom_header = AtomHeader {
                atom_type: AtomType::DtBlob as u8,
                count: 1,
                dlen: blob.len() as u16,
                reserved: 0,
            };
            let atom_ptr = &atom_header as *const _ as *const u8;
            bytes.extend_from_slice(unsafe {
                core::slice::from_raw_parts(atom_ptr, core::mem::size_of::<AtomHeader>())
            });
            bytes.extend_from_slice(blob);
        }

        // GPIO bank1
        if let Some(ref bank1) = self.gpio_map_bank1 {
            let atom_header = AtomHeader {
                atom_type: AtomType::GpioMapBank1 as u8,
                count: 1,
                dlen: core::mem::size_of::<GpioMapAtom>() as u16,
                reserved: 0,
            };
            let atom_ptr = &atom_header as *const _ as *const u8;
            bytes.extend_from_slice(unsafe {
                core::slice::from_raw_parts(atom_ptr, core::mem::size_of::<AtomHeader>())
            });
            let gpio_ptr = bank1 as *const _ as *const u8;
            bytes.extend_from_slice(unsafe {
                core::slice::from_raw_parts(gpio_ptr, core::mem::size_of::<GpioMapAtom>())
            });
        }

        // Custom atoms
        for (atom_type, data) in &self.custom_atoms {
            let atom_header = AtomHeader {
                atom_type: *atom_type,
                count: 1,
                dlen: data.len() as u16,
                reserved: 0,
            };
            let atom_ptr = &atom_header as *const _ as *const u8;
            bytes.extend_from_slice(unsafe {
                core::slice::from_raw_parts(atom_ptr, core::mem::size_of::<AtomHeader>())
            });
            bytes.extend_from_slice(data);
        }
    }

    /// Internal method to serialize into buffer (no_std version)
    pub fn serialize_to_buffer(
        &self,
        buffer: &mut [u8],
        offset: &mut usize,
    ) -> Result<(), EhatromError> {
        // Helper to copy data safely
        fn copy_to_buffer<T>(
            data: &T,
            buffer: &mut [u8],
            offset: &mut usize,
        ) -> Result<(), EhatromError> {
            let size = core::mem::size_of::<T>();
            if *offset + size > buffer.len() {
                return Err(EhatromError::BufferTooSmall);
            }
            let src = unsafe { core::slice::from_raw_parts(data as *const T as *const u8, size) };
            buffer[*offset..*offset + size].copy_from_slice(src);
            *offset += size;
            Ok(())
        }

        // EEPROM header
        copy_to_buffer(&self.header, buffer, offset)?;

        // VendorInfo
        let atom_header = AtomHeader {
            atom_type: AtomType::VendorInfo as u8,
            count: 1,
            dlen: core::mem::size_of::<VendorInfoAtom>() as u16,
            reserved: 0,
        };
        copy_to_buffer(&atom_header, buffer, offset)?;
        copy_to_buffer(&self.vendor_info, buffer, offset)?;

        // GPIO bank0
        let atom_header = AtomHeader {
            atom_type: AtomType::GpioMapBank0 as u8,
            count: 1,
            dlen: core::mem::size_of::<GpioMapAtom>() as u16,
            reserved: 0,
        };
        copy_to_buffer(&atom_header, buffer, offset)?;
        copy_to_buffer(&self.gpio_map_bank0, buffer, offset)?;

        // DT blob
        if let Some(ref blob) = self.dt_blob {
            let atom_header = AtomHeader {
                atom_type: AtomType::DtBlob as u8,
                count: 1,
                dlen: blob.len() as u16,
                reserved: 0,
            };
            copy_to_buffer(&atom_header, buffer, offset)?;
            if *offset + blob.len() > buffer.len() {
                return Err(EhatromError::BufferTooSmall);
            }
            buffer[*offset..*offset + blob.len()].copy_from_slice(blob);
            *offset += blob.len();
        }

        // GPIO bank1
        if let Some(ref bank1) = self.gpio_map_bank1 {
            let atom_header = AtomHeader {
                atom_type: AtomType::GpioMapBank1 as u8,
                count: 1,
                dlen: core::mem::size_of::<GpioMapAtom>() as u16,
                reserved: 0,
            };
            copy_to_buffer(&atom_header, buffer, offset)?;
            copy_to_buffer(bank1, buffer, offset)?;
        }

        // Custom atoms
        #[cfg(feature = "alloc")]
        for (atom_type, data) in &self.custom_atoms {
            let atom_header = AtomHeader {
                atom_type: *atom_type,
                count: 1,
                dlen: data.len() as u16,
                reserved: 0,
            };
            copy_to_buffer(&atom_header, buffer, offset)?;
            if *offset + data.len() > buffer.len() {
                return Err(EhatromError::BufferTooSmall);
            }
            buffer[*offset..*offset + data.len()].copy_from_slice(data);
            *offset += data.len();
        }

        #[cfg(not(feature = "alloc"))]
        for (atom_type, data) in self.custom_atoms {
            let atom_header = AtomHeader {
                atom_type: *atom_type,
                count: 1,
                dlen: data.len() as u16,
                reserved: 0,
            };
            copy_to_buffer(&atom_header, buffer, offset)?;
            if *offset + data.len() > buffer.len() {
                return Err(EhatromError::BufferTooSmall);
            }
            buffer[*offset..*offset + data.len()].copy_from_slice(data);
            *offset += data.len();
        }

        Ok(())
    }

    pub fn set_version(&mut self, version: u8) {
        self.header.version = version;
    }
}

impl From<u8> for AtomType {
    fn from(val: u8) -> Self {
        match val {
            0x01 => AtomType::VendorInfo,
            0x02 => AtomType::GpioMapBank0,
            0x03 => AtomType::DtBlob,
            0x04 => AtomType::GpioMapBank1,
            _ => AtomType::Unknown,
        }
    }
}

#[cfg(all(feature = "linux", any(target_os = "linux", target_os = "android")))]
pub fn write_to_eeprom_i2c(data: &[u8], dev_path: &str, addr: u16) -> Result<(), EhatromError> {
    // Validate EEPROM data before writing
    if data.len() < 12 {
        return Err(EhatromError::InvalidData);
    }

    // Check signature "R-Pi"
    if &data[0..4] != b"R-Pi" {
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
    if eeplen < 12 {
        return Err(EhatromError::InvalidData);
    }
    // Check CRC if available (we expect it to be the last 4 bytes)
    if data.len() >= 16 && !Eeprom::verify_crc(data) {
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

#[cfg(all(feature = "linux", any(target_os = "linux", target_os = "android")))]
pub fn read_from_eeprom_i2c(
    buf: &mut [u8],
    dev_path: &str,
    addr: u16,
    offset: u16,
) -> Result<(), EhatromError> {
    let mut dev = LinuxI2CDevice::new(dev_path, addr).map_err(|_| EhatromError::I2cError)?;
    // Send 2-byte offset first (high byte, low byte)
    let offset_bytes = [(offset >> 8) as u8, (offset & 0xFF) as u8];
    dev.write(&offset_bytes)
        .map_err(|_| EhatromError::I2cError)?;
    dev.read(buf).map_err(|_| EhatromError::I2cError)?;
    Ok(())
}

#[cfg(all(feature = "linux", any(target_os = "linux", target_os = "android")))]
pub mod detect;
#[cfg(all(feature = "linux", any(target_os = "linux", target_os = "android")))]
pub use detect::{detect_all_i2c_devices, detect_and_show_eeprom_info, find_i2c_devices};

impl VendorInfoAtom {
    /// Creates VendorInfoAtom from strings (automatically trims/pads with zeros)
    pub fn new(
        vendor_id: u16,
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
        let vendor_len = vendor_bytes.len().min(15); // leave space for null-terminator
        let product_len = product_bytes.len().min(15);
        vendor_arr[..vendor_len].copy_from_slice(&vendor_bytes[..vendor_len]);
        product_arr[..product_len].copy_from_slice(&product_bytes[..product_len]);
        // null-terminator is already present, as arrays are zero-filled
        VendorInfoAtom {
            vendor_id,
            product_id,
            product_ver,
            vendor: vendor_arr,
            product: product_arr,
            uuid,
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
