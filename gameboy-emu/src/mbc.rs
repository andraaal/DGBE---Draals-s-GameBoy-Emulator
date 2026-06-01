use crate::{header::Header, memory::MemoryError};

pub mod mbc1;
pub mod nmbc;

pub(crate) enum MBCType {
    NMBC(nmbc::NMBC),
    MBC1(mbc1::MBC1),
}

impl MBCType {
    pub fn new(rom: Vec<u8>, header: &Header) -> Result<Self, MemoryError> {
        match header.cart_type {
            0x00 => Ok(MBCType::NMBC(nmbc::NMBC::new(rom, 0)?)),
            0x01..=0x03 => Ok(MBCType::MBC1(mbc1::MBC1::new(rom, header.ram_banks)?)),
            _ => Err(format!(
                "Unsupported cartridge type: {:02X}",
                header.cart_type
            )),
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        match self {
            MBCType::NMBC(mbc) => mbc.read(address),
            MBCType::MBC1(mbc) => mbc.read(address),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match self {
            MBCType::NMBC(mbc) => mbc.write(address, value),
            MBCType::MBC1(mbc) => mbc.write(address, value),
        }
    }
}
