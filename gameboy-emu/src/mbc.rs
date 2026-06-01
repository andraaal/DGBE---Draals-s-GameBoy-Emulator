pub mod mbc1;
pub mod nmbc;

pub(crate) enum MBCType {
    NMBC(nmbc::NMBC),
    MBC1(mbc1::MBC1),
}

impl MBCType {
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
