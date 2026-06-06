mod background_fifo;

use background_fifo::BackgroundFIFO;
use std::collections::VecDeque;

pub(crate) struct PPU {
    // General state
    scanline: u8,
    dot: u16,
    frame: u64,
    delay: usize,
    mode: PPUState,

    // OAM Scan state
    oam_scan: [[u8; 4]; 10],
    oams_index: usize,
    oams_used_last: bool,

    // Drawing state
    bgw_fifo: BackgroundFIFO, // Background and Window FIFO state
    obj_fifo: BackgroundFIFO, // Object FIFO state
    x_coordinate: u8,
    y_coordinate: u8,
    window_y_cond: bool,
    window_fetching: bool,
}

impl PPU {
    pub fn new() -> Self {
        Self {
            scanline: 0,
            dot: 0,
            frame: 0,
            delay: 0,
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
        if self.delay > 0 {
            self.delay -= 1;
            return;
        }

        self.exec_dot(memory);

        self.dot += 1;
        if self.dot == 456 {
            self.dot = 0;
            self.scanline += 1;
            if self.scanline == 154 {
                self.scanline = 0;
                self.frame += 1;
            }
            // Reset OAM scan state at the end of each scanline
            self.oams_index = 0;
            self.oams_used_last = false;
        }
    }

    fn exec_dot(&mut self, memory: &mut crate::memory::Memory) {
        // Extract the lcdc (LDC-Control) register
        let lcdc = LCDC::from_byte(memory.get_byte(0xFF40));

        if self.scanline >= 144 {
            // VBlank
            self.mode = PPUState::VBlank;
            memory.oam_access = true;
            memory.vram_access = true;
        } else if self.dot <= 80 {
            // OAM Scan
            self.mode = PPUState::OAMScan;

            if self.oams_index >= 10 {
                // No more than 10 sprites can be rendered per scanline, so skip the rest of OAM scan
                return;
            }

            memory.oam_access = false;
            if self.dot % 2 == 0 {
                // Read first two bytes of Object Attribute
                let b0 = memory.get_byte(0xFE00 + (self.dot * 2) as u16);
                let b1 = memory.get_byte(0xFE00 + (self.dot * 2 + 1) as u16);

                if b0 <= self.scanline && self.scanline < b0 + if lcdc.obj_size { 16 } else { 8 } {
                    // Sprite is visible on this scanline, store it for later rendering
                    self.oam_scan[self.oams_index][0] = b0; // Y position
                    self.oam_scan[self.oams_index][1] = b1; // X position
                    self.oams_used_last = true;
                }
            } else {
                if self.oams_used_last {
                    // Read the remaining two bytes of Object Attribute if it was conclueded that the sprite is visible last dot (see code above)
                    let b2 = memory.get_byte(0xFE00 + (self.dot * 2) as u16);
                    let b3 = memory.get_byte(0xFE00 + (self.dot * 2 + 1) as u16);

                    self.oam_scan[self.oams_index][2] = b2; // Tile index
                    self.oam_scan[self.oams_index][3] = b3; // Attributes
                    self.oams_index += 1;
                } else {
                    // This is the second part of an OAM entry, but it was concluded that the sprite is not visible last dot (see code above)
                    return;
                }
            }
        } else if true {
            // Drawing
            self.mode = PPUState::Drawing;
            memory.oam_access = false;
            memory.vram_access = false;

            if !self.bgw_fifo.is_empty() && !self.obj_fifo.is_empty() {
                todo!(); // Draw pixel
                let bgw_pixel = self.bgw_fifo.pop_front().unwrap();
                let obj_pixel = self.obj_fifo.pop_front().unwrap();

                // If obj pixel is transparent => draw bg/w pixel
                // If bg-to-obj priority is 1 and bg/w pixel is not color 0 => draw bg/w pixel
                // Else draw obj pixel

                // Check if window should be drawn next
                if lcdc.window_display_enable
                    && !self.window_fetching
                    && self.window_y_cond
                    && self.x_coordinate >= memory.get_byte(0xFF4B) - 7
                {
                    self.window_fetching = true;
                    self.bgw_fifo.reset();
                }
            }
        }
    }
}

/// Describes the current mode of the PPU.
pub(crate) enum PPUState {
    OAMScan,
    Drawing,
    HBlank,
    VBlank,
}

/// Helper struct to decode the LCDC register (0xFF40)
struct LCDC {
    pub lcd_enable: bool,
    pub window_tile_map_select: bool,
    pub window_display_enable: bool,
    pub bgw_tile_data_select: bool,
    pub bg_tile_map_select: bool,
    pub obj_size: bool,
    pub obj_display_enable: bool,
    pub bgw_display: bool,
}

impl LCDC {
    pub fn from_byte(byte: u8) -> Self {
        Self {
            lcd_enable: byte & 0x80 != 0,
            window_tile_map_select: byte & 0x40 != 0,
            window_display_enable: byte & 0x20 != 0,
            bgw_tile_data_select: byte & 0x10 != 0,
            bg_tile_map_select: byte & 0x08 != 0,
            obj_size: byte & 0x04 != 0,
            obj_display_enable: byte & 0x02 != 0,
            bgw_display: byte & 0x01 != 0,
        }
    }
}
