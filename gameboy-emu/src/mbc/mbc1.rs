use crate::memory::MemoryError;

/// MBC1 cartridge. Supports up to 2 MiB of ROM and up to 32 KiB of RAM. Banks can be switched by writing to the ROM.
pub(crate) struct MBC1 {
    rom: Vec<[u8; 0x4000]>,
    ram: Vec<[u8; 0x2000]>,
    rom_bank1: usize,
    rom_bank2: usize,
    ram_bank: usize,
    ram_enabled: bool,
    rom_register: u8,
    ram_register: u8,
    advanced_banking_mode: bool,
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
        let initial_rom = rom
            .chunks(0x4000)
            .map(|chunk| {
                let mut bank = [0u8; 0x4000];
                bank[..chunk.len()].copy_from_slice(chunk);
                bank
            })
            .collect::<Vec<[u8; 0x4000]>>();

        let mut initial_ram = Vec::new();
        initial_ram.resize_with(ram, || [0u8; 0x2000]);

        Ok(Self {
            rom: initial_rom,
            ram: initial_ram,
            rom_bank1: 0,
            rom_bank2: 1,
            ram_bank: 0,
            ram_enabled: false,
            rom_register: 0x1,
            ram_register: 0x0,
            advanced_banking_mode: false,
        })
    }

    pub fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => {
                if self.advanced_banking_mode {
                    self.rom[self.rom_bank1][address as usize]
                } else {
                    self.rom[0][address as usize]
                }
            }
            0x4000..=0x7FFF => self.rom[self.rom_bank2][(address - 0x4000) as usize],
            0xA000..=0xBFFF => {
                if self.ram_enabled {
                    if self.advanced_banking_mode {
                        self.ram[self.ram_bank][(address - 0xA000) as usize]
                    } else {
                        self.ram[0][(address - 0xA000) as usize]
                    }
                } else {
                    0xFF
                }
            }
            _ => panic!(
                "Tried to read address {:04X} on cartridge: This address is found on the main gameboy",
                address
            ),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x1FFF => self.ram_enabled = value & 0x0F == 0x0A,
            0x2000..=0x3FFF => {
                self.rom_register = value & 0x1F;
                if self.rom_register == 0 {
                    self.rom_register = 1;
                }

                let full_address = self.rom_register | (self.ram_register << 5);
                self.rom_bank2 = (full_address & (self.rom.len() as u8 - 1)) as usize;
            }
            0x4000..=0x5FFF => {
                self.ram_register = value & 0x03;
                if self.advanced_banking_mode {
                    if self.ram.len() > 1 {
                        self.ram_bank = (self.ram_register & (self.ram.len() as u8 - 1)) as usize;
                    } else {
                        self.rom_bank2 = (self.ram_register << 5) as usize;
                    }
                    let full_address = self.rom_register | (self.ram_register << 5);
                    self.rom_bank2 = (full_address & (self.rom.len() as u8 - 1)) as usize;
                } else {
                    let full_address = self.rom_register | (self.ram_register << 5);
                    self.rom_bank2 = (full_address & (self.rom.len() as u8 - 1)) as usize;
                }
            }
            0x6000..=0x7FFF => {
                self.advanced_banking_mode = value & 0x01 == 0x01;
            }
            0xA000..=0xBFFF => self.ram[0][(address - 0xA000) as usize] = value,
            _ => panic!(
                "Tried to write to address {:04X} on cartridge: This address is found on the main gameboy",
                address
            ),
        }
    }
}
