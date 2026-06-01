use crate::memory::MemoryError;

/// Non-MBC cartridge. Thus only supports up to 32 KiB of ROM and up to 8 KiB of RAM as that is the maximum amount of memory that fits into the address space without bank switching.
pub(crate) struct NMBC {
    rom_banks: [[u8; 0x4000]; 2],
    ram_bank: Option<[u8; 0x2000]>,
}

impl NMBC {
    pub fn new(rom: Vec<u8>, ram: usize) -> Result<Self, MemoryError> {
        if rom.len() != 0x8000 {
            return Err(format!(
                "ROM size for NMBC must be exactly 32 KiB, was {:04}",
                rom.len()
            ));
        }

        let mut ram_banks = None;
        if ram == 1 {
            ram_banks = Some([0; 0x2000]);
        } else if ram > 1 {
            return Err(format!(
                "NMBC can only support up to 8 KiB of RAM (1 bank), but {} banks were requested",
                ram
            ));
        }

        Ok(Self {
            rom_banks: [[0; 0x4000]; 2],
            ram_bank: ram_banks,
        })
    }

    pub fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => self.rom_banks[0][address as usize],
            0x4000..=0x7FFF => self.rom_banks[1][(address - 0x4000) as usize],
            0xA000..=0xBFFF => self
                .ram_bank
                .map_or(0xFF, |a| a[(address - 0xA000) as usize]),
            _ => panic!(
                "Tried to read address {:04X} on cartridge: This address is found on the main gameboy",
                address
            ),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0xA000..=0xBFFF => {
                if let Some(ref mut a) = self.ram_bank {
                    a[(address - 0xA000) as usize] = value;
                }
            }
            _ => panic!(
                "Tried to write to address {:04X} on cartridge: This address is found on the main gameboy",
                address
            ),
        }
    }
}
