#[derive(Debug, Clone, Copy)]
pub struct Registers {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub f: u8,   // Flags register
    pub pc: u16, // Program Counter
    pub sp: u16, // Stack Pointer
}

impl Registers {
    pub fn new() -> Self {
        Self {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            f: 0,
            pc: 0x0000,
            sp: 0x0000,
        }
    }

    // Combined register access methods
    pub fn get_af(&self) -> u16 {
        ((self.a as u16) << 8) | (self.f as u16)
    }

    pub fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.f = (value as u8) & 0xF0;
    }

    pub fn get_bc(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }

    pub fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = value as u8;
    }

    pub fn get_de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }

    pub fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = value as u8;
    }

    pub fn get_hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }

    pub fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = value as u8;
    }

    // Flag manipulation methods
    pub fn set_z_flag(&mut self, value: bool) {
        self.f = (value as u8) << 7 | (self.f & 0b01111111);
    }

    pub fn get_z_flag(&self) -> bool {
        (self.f & 0b10000000) != 0
    }

    pub fn set_n_flag(&mut self, value: bool) {
        self.f = (value as u8) << 6 | (self.f & 0b10111111);
    }

    pub fn get_n_flag(&self) -> bool {
        (self.f & 0b01000000) != 0
    }

    pub fn set_h_flag(&mut self, value: bool) {
        self.f = (value as u8) << 5 | (self.f & 0b11011111);
    }

    pub fn get_h_flag(&self) -> bool {
        (self.f & 0b00100000) != 0
    }

    pub fn set_c_flag(&mut self, value: bool) {
        self.f = (value as u8) << 4 | (self.f & 0b11101111);
    }

    pub fn get_c_flag(&self) -> bool {
        (self.f & 0b00010000) != 0
    }
}
