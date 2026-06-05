pub(crate) struct PPU {
    // General state
    scanline: u8,
    dot: u16,
    frame: u64,
    mode: PPUState,

    // OAM Scan state
    oam_scan: [[u8; 4]; 10],
    oams_index: usize,
    oams_used_last: bool,
}

impl PPU {
    pub fn new() -> Self {
        Self {
            scanline: 0,
            dot: 0,
            frame: 0,
            mode: PPUState::OAMScan,
            oam_scan: [[0; 4]; 10],
            oams_index: 0,
            oams_used_last: false,
        }
    }

    pub fn step_cycle(&mut self, memory: &mut crate::memory::Memory) {
        for _ in 0..3 {
            self.step_dot(memory);
        }
    }

    pub fn step_dot(&mut self, memory: &mut crate::memory::Memory) {
        if self.scanline >= 144 {
            self.mode = PPUState::VBlank;
            // VBlank, do nothing for now
        } else if self.dot <= 80 {
            if self.oams_index >= 10 {
                // No more than 10 sprites can be rendered per scanline, so skip the rest of OAM scan
                return;
            }

            self.mode = PPUState::OAMScan;
            memory.oam_access = false;
            if self.dot % 2 == 0 {
                // Read first two bytes of Object Attribute
                let b1 = memory.get_byte(0xFE00 + (self.dot * 2) as u16);
                let b2 = memory.get_byte(0xFE00 + (self.dot * 2 + 1) as u16);

                
            }
        }
    }
}

pub(crate) enum PPUState {
    OAMScan,
    Drawing,
    HBlank,
    VBlank,
}
