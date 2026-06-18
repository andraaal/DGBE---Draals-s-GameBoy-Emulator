use crate::memory::Memory;
use crate::ppu::PPU;
use crate::registers::Registers;

pub(crate) struct CPU {
    memory: Memory,
    ppu: PPU,
    pub cycles: u64,
    reg: Registers,
    executed_opcodes: Vec<(u16, u8)>,
    errors: Vec<String>,
}

impl CPU {
    pub(crate) fn new(memory: Memory) -> Self {
        Self {
            memory,
            ppu: PPU::new(),
            cycles: 0,
            reg: Registers::new(),
            executed_opcodes: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub(crate) fn load_rom(&mut self, rom: Vec<u8>) -> Result<(), String> {
        if let Err(e) = self.memory.load_rom(rom) {
            let error = Err(format!("Failed to load ROM: {}", e));
            self.errors.push(e);
            error
        } else {
            Ok(())
        }
    }

    pub(crate) fn debug_view(&mut self) -> crate::DebugView {
        let mut errors = std::mem::take(&mut self.errors);
        errors.append(&mut self.memory.take_errors());
        // NOTE: Should sort errors chronologically, and not just append; Probably won't matter since this system has to go anyways

        crate::DebugView {
            cycles: self.cycles,
            registers: self.reg,
            opcodes: std::mem::take(&mut self.executed_opcodes),
            errors: errors,
            framebuffer: self.ppu.get_framebuffer().clone(),
        }
    }

    pub(crate) fn execute(&mut self) {
        loop {
            self.step();
            if self.errors.len() > 0 {
                return;
            }
        }
    }

    pub(crate) fn step(&mut self) {
        // Check for Interrupts
        let i_enable = self.memory.read_byte(0xFFFF);
        let i_flag = self.memory.read_byte(0xFF0F);
        let inter = i_enable & i_flag;
        if self.reg.ime {
            if inter & 0x01 != 0 {
                // V-Blank Interrupt
                self.handle_interrupt(0x40);
                self.memory.write_byte(0xFF0F, i_flag & !0x01);
            } else if inter & 0x02 != 0 {
                // LCD STAT Interrupt
                self.handle_interrupt(0x48);
                self.memory.write_byte(0xFF0F, i_flag & !0x02);
            } else if inter & 0x04 != 0 {
                // Timer Interrupt
                self.handle_interrupt(0x50);
                self.memory.write_byte(0xFF0F, i_flag & !0x04);
            } else if inter & 0x08 != 0 {
                // Serial Interrupt
                self.handle_interrupt(0x58);
                self.memory.write_byte(0xFF0F, i_flag & !0x08);
            } else if inter & 0x10 != 0 {
                // Joypad Interrupt
                self.handle_interrupt(0x60);
                self.memory.write_byte(0xFF0F, i_flag & !0x10);
            }
        }

        if self.reg.ime_next {
            self.reg.ime = true;
            self.reg.ime_next = false;
        }

        let location = self.reg.pc;
        let opcode = self.fetch_byte();
        self.executed_opcodes.push((location, opcode));

        let duration = match opcode {
            // Miscellaneous Instructions
            0x00 => 1, // NOP
            0xCB => {
                // Prefix for bit operations
                self.execute_opcode_extension()
            }

            // Load Instructions (LD <location>,<value>)

            // 16bit Load Instructions
            0x01 => {
                // LD BC,d16
                let value = self.fetch_word();
                self.reg.set_bc(value);
                3
            }
            0x11 => {
                // LD DE,d16
                let value = self.fetch_word();
                self.reg.set_de(value);
                3
            }
            0x21 => {
                // LD HL,d16
                let value = self.fetch_word();
                self.reg.set_hl(value);
                3
            }
            0x31 => {
                // LD SP,d16
                let value = self.fetch_word();
                self.reg.sp = value;
                3
            }
            0x08 => {
                // LD (a16),SP
                let address = self.fetch_word();
                self.memory.write_word(address, &self.reg.sp.to_le_bytes());
                5
            }

            // 8bit Load Instructions between registers
            0x40..=0x7F if opcode != 0x76 => {
                // LD r8,r8
                let dest = (opcode >> 3) & 0x07;
                let source = opcode & 0x07;
                let value = self.read_r8(source);
                self.write_r8(dest, value);
                if source == 0x06 || dest == 0x06 { 2 } else { 1 }
            }

            // 8bit Load Instructions between registers and constants
            0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x36 | 0x3E => {
                // LD r8,d8 or LD (HL),d8
                let value = self.fetch_byte();
                self.write_r8(opcode >> 3, value);
                if opcode == 0x36 { 3 } else { 2 }
            }

            // 8bit Load Instructions between registers and memory
            0x02 => {
                // LD (BC),A
                self.memory.write_byte(self.reg.get_bc(), self.reg.a);
                2
            }
            0x12 => {
                // LD (DE),A
                self.memory.write_byte(self.reg.get_de(), self.reg.a);
                2
            }
            0x0A => {
                // LD A,(BC)
                self.reg.a = self.memory.read_byte(self.reg.get_bc());
                2
            }
            0x1A => {
                // LD A,(DE)
                self.reg.a = self.memory.read_byte(self.reg.get_de());
                2
            }
            0x22 => {
                // LD (HL+),A
                let hl = self.reg.get_hl();
                self.memory.write_byte(hl, self.reg.a);
                self.reg.set_hl(hl.wrapping_add(1));
                2
            }
            0x32 => {
                // LD (HL-),A
                let hl = self.reg.get_hl();
                self.memory.write_byte(hl, self.reg.a);
                self.reg.set_hl(hl.wrapping_sub(1));
                2
            }
            0x2A => {
                // LD A,(HL+)
                let hl = self.reg.get_hl();
                self.reg.a = self.memory.read_byte(hl);
                self.reg.set_hl(hl.wrapping_add(1));
                2
            }
            0x3A => {
                // LD A,(HL-)
                let hl = self.reg.get_hl();
                self.reg.a = self.memory.read_byte(hl);
                self.reg.set_hl(hl.wrapping_sub(1));
                2
            }
            0xE0 => {
                // LDH (a8),A
                let address = 0xFF00 + self.fetch_byte() as u16;
                self.memory.write_byte(address, self.reg.a);
                3
            }
            0xE2 => {
                // LD (C+0xFF00),A
                let address = 0xFF00 + self.reg.c as u16;
                self.memory.write_byte(address, self.reg.a);
                2
            }
            0xEA => {
                // LD (a16),A
                let address = self.fetch_word();
                self.memory.write_byte(address, self.reg.a);
                4
            }
            0xF0 => {
                // LDH A,(a8)
                let address = 0xFF00 + self.fetch_byte() as u16;
                self.reg.a = self.memory.read_byte(address);
                3
            }
            0xF2 => {
                // LD A,(C+0xFF00)
                let address = 0xFF00 + self.reg.c as u16;
                self.reg.a = self.memory.read_byte(address);
                2
            }
            0xFA => {
                // LD A,(a16)
                let address = self.fetch_word();
                self.reg.a = self.memory.read_byte(address);
                4
            }

            // 16bit Load Instructions between registers and memory
            0xF8 => {
                // LD HL,SP+r8
                let offset = self.fetch_byte() as i8 as i16 as u16;
                let result = self.reg.sp.wrapping_add(offset);
                self.reg.set_hl(result);
                self.reg.set_z_flag(false);
                self.reg.set_n_flag(false);
                self.reg
                    .set_h_flag(((self.reg.sp & 0x0F) + (offset & 0x0F)) > 0x0F);
                self.reg
                    .set_c_flag(((self.reg.sp & 0xFF) + (offset & 0xFF)) > 0xFF);
                3
            }
            0xF9 => {
                // LD SP,HL
                self.reg.sp = self.reg.get_hl();
                2
            }

            // Arithmetic Instructions
            // 16bit arithmetic instructions
            0x03 => {
                // INC BC
                self.reg.set_bc(self.reg.get_bc().wrapping_add(1));
                2
            }
            0x13 => {
                // INC DE
                self.reg.set_de(self.reg.get_de().wrapping_add(1));
                2
            }
            0x23 => {
                // INC HL
                self.reg.set_hl(self.reg.get_hl().wrapping_add(1));
                2
            }
            0x33 => {
                // INC SP
                self.reg.sp = self.reg.sp.wrapping_add(1);
                2
            }
            0x0B => {
                // DEC BC
                self.reg.set_bc(self.reg.get_bc().wrapping_sub(1));
                2
            }
            0x1B => {
                // DEC DE
                self.reg.set_de(self.reg.get_de().wrapping_sub(1));
                2
            }
            0x2B => {
                // DEC HL
                self.reg.set_hl(self.reg.get_hl().wrapping_sub(1));
                2
            }
            0x3B => {
                // DEC SP
                self.reg.sp = self.reg.sp.wrapping_sub(1);
                2
            }
            0x09 => {
                // ADD HL,BC
                self.add_to_hl(self.reg.get_bc());
                2
            }
            0x19 => {
                // ADD HL,DE
                self.add_to_hl(self.reg.get_de());
                2
            }
            0x29 => {
                // ADD HL,HL
                self.add_to_hl(self.reg.get_hl());
                2
            }
            0x39 => {
                // ADD HL,SP
                self.add_to_hl(self.reg.sp);
                2
            }
            0xE8 => {
                // ADD SP,r8
                let offset = self.fetch_byte() as i8 as i16 as u16;
                let sp = self.reg.sp;
                self.reg.sp = sp.wrapping_add(offset);
                self.reg.set_z_flag(false);
                self.reg.set_n_flag(false);
                self.reg.set_h_flag(((sp & 0x0F) + (offset & 0x0F)) > 0x0F);
                self.reg.set_c_flag(((sp & 0xFF) + (offset & 0xFF)) > 0xFF);
                4
            }

            // 8bit arithmetic instructions
            0x04 => {
                // INC B
                self.reg.b = self.inc_u8(self.reg.b);
                1
            }
            0x0C => {
                // INC C
                self.reg.c = self.inc_u8(self.reg.c);
                1
            }
            0x14 => {
                // INC D
                self.reg.d = self.inc_u8(self.reg.d);
                1
            }
            0x1C => {
                // INC E
                self.reg.e = self.inc_u8(self.reg.e);
                1
            }
            0x24 => {
                // INC H
                self.reg.h = self.inc_u8(self.reg.h);
                1
            }
            0x2C => {
                // INC L
                self.reg.l = self.inc_u8(self.reg.l);
                1
            }
            0x34 => {
                // INC (HL)
                let hl = self.reg.get_hl();
                let value = self.memory.read_byte(hl);
                let incremented = self.inc_u8(value);
                self.memory.write_byte(hl, incremented);
                3
            }
            0x3C => {
                // INC A
                self.reg.a = self.inc_u8(self.reg.a);
                1
            }
            0x05 => {
                // DEC B
                self.reg.b = self.dec_u8(self.reg.b);
                1
            }
            0x0D => {
                // DEC C
                self.reg.c = self.dec_u8(self.reg.c);
                1
            }
            0x15 => {
                // DEC D
                self.reg.d = self.dec_u8(self.reg.d);
                1
            }
            0x1D => {
                // DEC E
                self.reg.e = self.dec_u8(self.reg.e);
                1
            }
            0x25 => {
                // DEC H
                self.reg.h = self.dec_u8(self.reg.h);
                1
            }
            0x2D => {
                // DEC L
                self.reg.l = self.dec_u8(self.reg.l);
                1
            }
            0x35 => {
                // DEC (HL)
                let hl = self.reg.get_hl();
                let value = self.memory.read_byte(hl);
                let decremented = self.dec_u8(value);
                self.memory.write_byte(hl, decremented);
                3
            }
            0x3D => {
                // DEC A
                self.reg.a = self.dec_u8(self.reg.a);
                1
            }
            0x80..=0xBF => {
                let source = opcode & 0x07;
                let value = self.read_r8(source);
                match opcode & 0xF8 {
                    0x80 => self.add_to_a(value, false),   // ADD A,r8
                    0x88 => self.add_to_a(value, true),    // ADC A,r8
                    0x90 => self.sub_from_a(value, false), // SUB A,r8
                    0x98 => self.sub_from_a(value, true),  // SBC A,r8
                    0xA0 => self.and_a(value),             // AND A,r8
                    0xA8 => self.xor_a(value),             // XOR A,r8
                    0xB0 => self.or_a(value),              // OR A,r8
                    0xB8 => self.cp_a(value),              // CP A,r8
                    _ => unreachable!(),
                }
                if source == 0x06 { 2 } else { 1 }
            }
            0xC6 => {
                // ADD A,d8
                let value = self.fetch_byte();
                self.add_to_a(value, false);
                2
            }
            0xCE => {
                // ADC A,d8
                let value = self.fetch_byte();
                self.add_to_a(value, true);
                2
            }
            0xD6 => {
                // SUB A,d8
                let value = self.fetch_byte();
                self.sub_from_a(value, false);
                2
            }
            0xDE => {
                // SBC A,d8
                let value = self.fetch_byte();
                self.sub_from_a(value, true);
                2
            }
            0xE6 => {
                // AND A,d8
                let value = self.fetch_byte();
                self.and_a(value);
                2
            }
            0xEE => {
                // XOR A,d8
                let value = self.fetch_byte();
                self.xor_a(value);
                2
            }
            0xF6 => {
                // OR A,d8
                let value = self.fetch_byte();
                self.or_a(value);
                2
            }
            0xFE => {
                // CP A,d8
                let value = self.fetch_byte();
                self.cp_a(value);
                2
            }
            0x27 => {
                // DAA
                self.daa();
                1
            }
            0x2F => {
                // CPL
                self.reg.a = !self.reg.a;
                self.reg.set_n_flag(true);
                self.reg.set_h_flag(true);
                1
            }
            0x37 => {
                // SCF
                self.reg.set_n_flag(false);
                self.reg.set_h_flag(false);
                self.reg.set_c_flag(true);
                1
            }
            0x3F => {
                // CCF
                self.reg.set_n_flag(false);
                self.reg.set_h_flag(false);
                self.reg.set_c_flag(!self.reg.get_c_flag());
                1
            }

            // Rotates and shifts (non-CB)
            0x07 => {
                // RLCA
                self.reg.a = self.rlc(self.reg.a, false);
                self.reg.set_z_flag(false);
                1
            }
            0x0F => {
                // RRCA
                self.reg.a = self.rrc(self.reg.a, false);
                self.reg.set_z_flag(false);
                1
            }
            0x17 => {
                // RLA
                self.reg.a = self.rl(self.reg.a, false);
                self.reg.set_z_flag(false);
                1
            }
            0x1F => {
                // RRA
                self.reg.a = self.rr(self.reg.a, false);
                self.reg.set_z_flag(false);
                1
            }

            // Jump / Call / Return / Restart instructions
            0x18 => {
                // JR r8
                let offset = self.fetch_byte() as i8;
                self.reg.pc = self.reg.pc.wrapping_add_signed(offset as i16);
                3
            }
            0x20 => {
                // JR NZ,r8
                self.jr_cond(!self.reg.get_z_flag())
            }
            0x28 => {
                // JR Z,r8
                self.jr_cond(self.reg.get_z_flag())
            }
            0x30 => {
                // JR NC,r8
                self.jr_cond(!self.reg.get_c_flag())
            }
            0x38 => {
                // JR C,r8
                self.jr_cond(self.reg.get_c_flag())
            }
            0xC2 => {
                // JP NZ,a16
                self.jp_cond(!self.reg.get_z_flag())
            }
            0xC3 => {
                // JP a16
                let address = self.fetch_word();
                self.reg.pc = address;
                4
            }
            0xCA => {
                // JP Z,a16
                self.jp_cond(self.reg.get_z_flag())
            }
            0xD2 => {
                // JP NC,a16
                self.jp_cond(!self.reg.get_c_flag())
            }
            0xDA => {
                // JP C,a16
                self.jp_cond(self.reg.get_c_flag())
            }
            0xE9 => {
                // JP HL
                self.reg.pc = self.reg.get_hl();
                1
            }
            0xC4 => {
                // CALL NZ,a16
                self.call_cond(!self.reg.get_z_flag())
            }
            0xCC => {
                // CALL Z,a16
                self.call_cond(self.reg.get_z_flag())
            }
            0xCD => {
                // CALL a16
                let address = self.fetch_word();
                self.push_word(self.reg.pc);
                self.reg.pc = address;
                6
            }
            0xD4 => {
                // CALL NC,a16
                self.call_cond(!self.reg.get_c_flag())
            }
            0xDC => {
                // CALL C,a16
                self.call_cond(self.reg.get_c_flag())
            }
            0xC0 => {
                // RET NZ
                self.ret_cond(!self.reg.get_z_flag())
            }
            0xC8 => {
                // RET Z
                self.ret_cond(self.reg.get_z_flag())
            }
            0xC9 => {
                // RET
                self.reg.pc = self.pop_word();
                4
            }
            0xD0 => {
                // RET NC
                self.ret_cond(!self.reg.get_c_flag())
            }
            0xD8 => {
                // RET C
                self.ret_cond(self.reg.get_c_flag())
            }
            0xD9 => {
                // RETI
                self.reg.pc = self.pop_word();
                self.reg.ime = true;
                4
            }
            0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => {
                // RST vec
                let target = (opcode & 0b0011_1000) as u16;
                self.push_word(self.reg.pc);
                self.reg.pc = target;
                4
            }

            // Stack instructions
            0xC1 => {
                // POP BC
                let value = self.pop_word();
                self.reg.set_bc(value);
                3
            }
            0xD1 => {
                // POP DE
                let value = self.pop_word();
                self.reg.set_de(value);
                3
            }
            0xE1 => {
                // POP HL
                let value = self.pop_word();
                self.reg.set_hl(value);
                3
            }
            0xF1 => {
                // POP AF
                let value = self.pop_word();
                self.reg.set_af(value);
                3
            }
            0xC5 => {
                // PUSH BC
                self.push_word(self.reg.get_bc());
                4
            }
            0xD5 => {
                // PUSH DE
                self.push_word(self.reg.get_de());
                4
            }
            0xE5 => {
                // PUSH HL
                self.push_word(self.reg.get_hl());
                4
            }
            0xF5 => {
                // PUSH AF
                self.push_word(self.reg.get_af());
                4
            }

            // Interrupt control
            0xF3 => {
                // DI
                self.reg.ime = false;
                self.reg.ime_next = false;
                1
            }
            0xFB => {
                // EI
                self.reg.ime_next = true;
                1
            }

            _ => {
                let message = format!(
                    "Unknown opcode: 0x{:02X} at address 0x{:04X}",
                    opcode,
                    self.reg.pc - 1
                );
                self.errors.push(message.clone());
                eprintln!("{}", message);
                0
            }
        };

        self.cycles += duration;

        // Step the PPU for the current cycle
        for _ in 0..duration {
            self.ppu.step_mcycle(&mut self.memory);
        }
    }

    fn handle_interrupt(&mut self, address: u16) {
        self.reg.ime = false;
        self.push_word(self.reg.pc);
        self.reg.pc = address;
        self.cycles += 5;

        // Step the PPU for the current cycle
        for _ in 0..5 {
            self.ppu.step_mcycle(&mut self.memory);
        }
    }

    fn execute_opcode_extension(&mut self) -> u64 {
        let opcode = self.fetch_byte();
        let register_index = opcode & 0x07;
        let mut value = self.read_r8(register_index);

        match opcode {
            0x00..=0x07 => value = self.rlc(value, true), // RLC r8
            0x08..=0x0F => value = self.rrc(value, true), // RRC r8
            0x10..=0x17 => value = self.rl(value, true),  // RL r8
            0x18..=0x1F => value = self.rr(value, true),  // RR r8
            0x20..=0x27 => value = self.sla(value),       // SLA r8
            0x28..=0x2F => value = self.sra(value),       // SRA r8
            0x30..=0x37 => value = self.swap(value),      // SWAP r8
            0x38..=0x3F => value = self.srl(value),       // SRL r8
            0x40..=0x7F => {
                // BIT b3,r8
                let bit = (opcode >> 3) & 0x07;
                self.bit(value, bit);
            }
            0x80..=0xBF => {
                // RES b3,r8
                let bit = (opcode >> 3) & 0x07;
                value &= !(1 << bit);
            }
            0xC0..=0xFF => {
                // SET b3,r8
                let bit = (opcode >> 3) & 0x07;
                value |= 1 << bit;
            }
        }

        // Since the value is unchanged for BIT operations, nothing will change
        self.write_r8(register_index, value);

        if register_index == 0x06 {
            if (0x40..=0x7F).contains(&opcode) {
                3
            } else {
                4
            }
        } else {
            2
        }
    }

    fn read_r8(&mut self, register_index: u8) -> u8 {
        match register_index {
            0 => self.reg.b,
            1 => self.reg.c,
            2 => self.reg.d,
            3 => self.reg.e,
            4 => self.reg.h,
            5 => self.reg.l,
            6 => self.memory.read_byte(self.reg.get_hl()),
            7 => self.reg.a,
            _ => unreachable!(),
        }
    }

    fn write_r8(&mut self, register_index: u8, value: u8) {
        match register_index {
            0 => self.reg.b = value,
            1 => self.reg.c = value,
            2 => self.reg.d = value,
            3 => self.reg.e = value,
            4 => self.reg.h = value,
            5 => self.reg.l = value,
            6 => self.memory.write_byte(self.reg.get_hl(), value),
            7 => self.reg.a = value,
            _ => unreachable!(),
        }
    }

    fn inc_u8(&mut self, value: u8) -> u8 {
        let result = value.wrapping_add(1);
        self.reg.set_z_flag(result == 0);
        self.reg.set_n_flag(false);
        self.reg.set_h_flag((value & 0x0F) == 0x0F);
        result
    }

    fn dec_u8(&mut self, value: u8) -> u8 {
        let result = value.wrapping_sub(1);
        self.reg.set_z_flag(result == 0);
        self.reg.set_n_flag(true);
        self.reg.set_h_flag((value & 0x0F) == 0);
        result
    }

    fn add_to_a(&mut self, value: u8, with_carry: bool) {
        let carry = if with_carry && self.reg.get_c_flag() {
            1
        } else {
            0
        };
        let a = self.reg.a;
        let result = a.wrapping_add(value).wrapping_add(carry);

        self.reg.set_z_flag(result == 0);
        self.reg.set_n_flag(false);
        self.reg
            .set_h_flag(((a & 0x0F) + (value & 0x0F) + carry) > 0x0F);
        self.reg
            .set_c_flag((a as u16) + (value as u16) + (carry as u16) > 0xFF);
        self.reg.a = result;
    }

    fn sub_from_a(&mut self, value: u8, with_carry: bool) {
        let carry = if with_carry && self.reg.get_c_flag() {
            1
        } else {
            0
        };
        let a = self.reg.a;
        let result = a.wrapping_sub(value).wrapping_sub(carry);

        self.reg.set_z_flag(result == 0);
        self.reg.set_n_flag(true);
        self.reg
            .set_h_flag((a & 0x0F) < ((value & 0x0F).wrapping_add(carry)));
        self.reg
            .set_c_flag((a as u16) < (value as u16) + (carry as u16));
        self.reg.a = result;
    }

    fn and_a(&mut self, value: u8) {
        self.reg.a &= value;
        self.reg.set_z_flag(self.reg.a == 0);
        self.reg.set_n_flag(false);
        self.reg.set_h_flag(true);
        self.reg.set_c_flag(false);
    }

    fn xor_a(&mut self, value: u8) {
        self.reg.a ^= value;
        self.reg.set_z_flag(self.reg.a == 0);
        self.reg.set_n_flag(false);
        self.reg.set_h_flag(false);
        self.reg.set_c_flag(false);
    }

    fn or_a(&mut self, value: u8) {
        self.reg.a |= value;
        self.reg.set_z_flag(self.reg.a == 0);
        self.reg.set_n_flag(false);
        self.reg.set_h_flag(false);
        self.reg.set_c_flag(false);
    }

    fn cp_a(&mut self, value: u8) {
        let a = self.reg.a;
        let result = a.wrapping_sub(value);
        self.reg.set_z_flag(result == 0);
        self.reg.set_n_flag(true);
        self.reg.set_h_flag((a & 0x0F) < (value & 0x0F));
        self.reg.set_c_flag(a < value);
    }

    fn add_to_hl(&mut self, value: u16) {
        let hl = self.reg.get_hl();
        let result = hl.wrapping_add(value);
        self.reg.set_n_flag(false);
        self.reg
            .set_h_flag(((hl & 0x0FFF) + (value & 0x0FFF)) > 0x0FFF);
        self.reg.set_c_flag((hl as u32 + value as u32) > 0xFFFF);
        self.reg.set_hl(result);
    }

    fn daa(&mut self) {
        let mut a = self.reg.a;
        let mut adjustment = 0;
        let mut carry = self.reg.get_c_flag();

        if self.reg.get_n_flag() {
            if self.reg.get_h_flag() {
                adjustment += 0x06;
            }
            if carry {
                adjustment += 0x60;
            }
            a = a.wrapping_sub(adjustment);
        } else {
            if self.reg.get_h_flag() || (a & 0x0F) > 0x09 {
                adjustment += 0x06;
            }
            if carry || a > 0x99 {
                adjustment += 0x60;
                carry = true;
            }
            a = a.wrapping_add(adjustment);
        }

        self.reg.a = a;
        self.reg.set_z_flag(a == 0);
        self.reg.set_h_flag(false);
        self.reg.set_c_flag(carry);
    }

    fn jr_cond(&mut self, condition: bool) -> u64 {
        let offset = self.fetch_byte() as i8;
        if condition {
            self.reg.pc = self.reg.pc.wrapping_add_signed(offset as i16);
            3
        } else {
            2
        }
    }

    fn jp_cond(&mut self, condition: bool) -> u64 {
        let address = self.fetch_word();
        if condition {
            self.reg.pc = address;
            4
        } else {
            3
        }
    }

    fn call_cond(&mut self, condition: bool) -> u64 {
        let address = self.fetch_word();
        if condition {
            self.push_word(self.reg.pc);
            self.reg.pc = address;
            6
        } else {
            3
        }
    }

    fn ret_cond(&mut self, condition: bool) -> u64 {
        if condition {
            self.reg.pc = self.pop_word();
            5
        } else {
            2
        }
    }

    fn push_word(&mut self, value: u16) {
        let [low, high] = value.to_le_bytes();
        self.reg.sp = self.reg.sp.wrapping_sub(1);
        self.memory.write_byte(self.reg.sp, high);
        self.reg.sp = self.reg.sp.wrapping_sub(1);
        self.memory.write_byte(self.reg.sp, low);
    }

    fn pop_word(&mut self) -> u16 {
        let low = self.memory.read_byte(self.reg.sp);
        self.reg.sp = self.reg.sp.wrapping_add(1);
        let high = self.memory.read_byte(self.reg.sp);
        self.reg.sp = self.reg.sp.wrapping_add(1);
        u16::from_le_bytes([low, high])
    }

    fn rlc(&mut self, value: u8, set_z: bool) -> u8 {
        let carry = (value & 0x80) != 0;
        let result = value.rotate_left(1);
        if set_z {
            self.reg.set_z_flag(result == 0);
        }
        self.reg.set_n_flag(false);
        self.reg.set_h_flag(false);
        self.reg.set_c_flag(carry);
        result
    }

    fn rrc(&mut self, value: u8, set_z: bool) -> u8 {
        let carry = (value & 0x01) != 0;
        let result = value.rotate_right(1);
        if set_z {
            self.reg.set_z_flag(result == 0);
        }
        self.reg.set_n_flag(false);
        self.reg.set_h_flag(false);
        self.reg.set_c_flag(carry);
        result
    }

    fn rl(&mut self, value: u8, set_z: bool) -> u8 {
        let carry_in = if self.reg.get_c_flag() { 1 } else { 0 };
        let carry_out = (value & 0x80) != 0;
        let result = (value << 1) | carry_in;
        if set_z {
            self.reg.set_z_flag(result == 0);
        }
        self.reg.set_n_flag(false);
        self.reg.set_h_flag(false);
        self.reg.set_c_flag(carry_out);
        result
    }

    fn rr(&mut self, value: u8, set_z: bool) -> u8 {
        let carry_in = if self.reg.get_c_flag() { 0x80 } else { 0 };
        let carry_out = (value & 0x01) != 0;
        let result = (value >> 1) | carry_in;
        if set_z {
            self.reg.set_z_flag(result == 0);
        }
        self.reg.set_n_flag(false);
        self.reg.set_h_flag(false);
        self.reg.set_c_flag(carry_out);
        result
    }

    fn sla(&mut self, value: u8) -> u8 {
        let carry = (value & 0x80) != 0;
        let result = value << 1;
        self.reg.set_z_flag(result == 0);
        self.reg.set_n_flag(false);
        self.reg.set_h_flag(false);
        self.reg.set_c_flag(carry);
        result
    }

    fn sra(&mut self, value: u8) -> u8 {
        let carry = (value & 0x01) != 0;
        let result = (value >> 1) | (value & 0x80);
        self.reg.set_z_flag(result == 0);
        self.reg.set_n_flag(false);
        self.reg.set_h_flag(false);
        self.reg.set_c_flag(carry);
        result
    }

    fn srl(&mut self, value: u8) -> u8 {
        let carry = (value & 0x01) != 0;
        let result = value >> 1;
        self.reg.set_z_flag(result == 0);
        self.reg.set_n_flag(false);
        self.reg.set_h_flag(false);
        self.reg.set_c_flag(carry);
        result
    }

    fn swap(&mut self, value: u8) -> u8 {
        let result = value.rotate_left(4);
        self.reg.set_z_flag(result == 0);
        self.reg.set_n_flag(false);
        self.reg.set_h_flag(false);
        self.reg.set_c_flag(false);
        result
    }

    fn bit(&mut self, value: u8, bit: u8) {
        self.reg.set_z_flag((value & (1 << bit)) == 0);
        self.reg.set_n_flag(false);
        self.reg.set_h_flag(true);
    }

    fn fetch_byte(&mut self) -> u8 {
        let byte = self.memory.read_byte(self.reg.pc);
        self.reg.pc = self.reg.pc.wrapping_add(1);
        byte
    }

    fn fetch_word(&mut self) -> u16 {
        let low = self.fetch_byte() as u16;
        let high = self.fetch_byte() as u16;
        (high << 8) | low
    }
}
