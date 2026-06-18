mod cpu;
mod header;
mod mbc;
mod memory;
mod ppu;
pub mod registers;

pub struct Emulator {
    cpu: crate::cpu::CPU,
}

#[derive(Debug, Clone)]
pub struct DebugView {
    /// The total number of cycles since the emulator started.
    pub cycles: u64,
    /// The current value of the registers.
    pub registers: registers::Registers,
    /// All opcodes and their index in memory executed since the last debug view was generated.
    pub opcodes: Vec<(u16, u8)>,
    /// Any errors encountered during execution (e.g., unknown opcodes).
    pub errors: Vec<String>,
    /// The framebuffer
    pub framebuffer: [[u8; 160]; 144],
}

impl Emulator {
    pub fn new() -> Self {
        let memory = crate::memory::Memory::new();
        let cpu = crate::cpu::CPU::new(memory);
        Self { cpu }
    }

    pub fn load_rom(&mut self, rom: Vec<u8>) -> Result<(), String> {
        self.cpu.load_rom(rom)
    }

    pub fn step(&mut self) -> DebugView {
        self.cpu.step();
        self.cpu.debug_view()
    }

    pub fn step_frame(&mut self) -> DebugView {
        for _ in 0..70224 {
            self.cpu.step();
        }
        self.cpu.debug_view()
    }

    pub fn run(&mut self) -> DebugView {
        self.cpu.execute();
        self.cpu.debug_view()
    }
}
