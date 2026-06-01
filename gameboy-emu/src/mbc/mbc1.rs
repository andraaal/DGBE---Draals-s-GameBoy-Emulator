use crate::memory::MemoryError;

/// MBC1 cartridge. Supports up to 2 MiB of ROM and up to 32 KiB of RAM. Banks can be switched by writing to the ROM.
pub(crate) struct MBC1 {
    rom: Vec<[u8; 0x4000]>,
    ram: Vec<[u8; 0x2000]>,
    rom_bank: usize,
    ram_bank: usize,
}

impl MBC1 {
    pub fn new(rom: Vec<u8>, ram: usize) -> Result<Self, MemoryError> {
        if rom.len() % 0x4000 != 0 {
            return Err(format!(
                "ROM size must be a multiple of 16 KiB (0x4000 bytes) and at least 32 KiB, but was {:04}",
                rom.len()
            ));
        }

        if ram > 4 || (ram > 1 && rom.len() > 0x80000) {
            return Err(format!(
                "MBC1 can only support up to 32 KiB of RAM (4 banks), but {} banks were requested. If the ROM size is larger than 512 KiB (0x80000 bytes), only 1 RAM bank is supported.",
                ram
            ));
        }

        #[expect(unreachable_code)]
        Ok(Self {
            rom: Vec::from(
                rom.chunks(0x4000)
                    .map(|chunk| {
                        let mut bank = [0u8; 0x4000];
                        bank[..chunk.len()].copy_from_slice(chunk);
                        bank
                    })
                    .collect::<Vec<[u8; 0x4000]>>(),
            ),
            ram: todo!(),
            rom_bank: 1,
            ram_bank: 0,
        })
    }

    pub fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => self.rom[0][address as usize],
            0x4000..=0x7FFF => self.rom[1][(address - 0x4000) as usize],
            0xA000..=0xBFFF => self.ram[0][(address - 0xA000) as usize],
            _ => panic!(
                "Tried to read address {:04X} on cartridge: This address is found on the main gameboy",
                address
            ),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0xA000..=0xBFFF => self.ram[0][(address - 0xA000) as usize] = value,
            _ => panic!(
                "Tried to write to address {:04X} on cartridge: This address is found on the main gameboy",
                address
            ),
        }
    }
}
