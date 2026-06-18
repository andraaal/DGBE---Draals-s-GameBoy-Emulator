mod background_fifo;

use crate::memory::Memory;
use background_fifo::BackgroundFIFO;

pub(crate) struct PPU {
    // General state
    // Note to self: Everything is 0-based
    /// Frame number
    frame: u64,
    /// Currently drawn line
    scanline: u8,
    /// Currently drawn dot in line
    dot: u8,
    /// Current t-cycle in line, resets after 456 t-cycles
    tcycle: u16,
    /// Delay counter in t-cycles
    delay: usize,
    /// Current PPU mode
    mode: PPUState,

    // OAM Scan state
    /// The sprites that are selected for rendering during OAM are stored here
    oam_scan: [[u8; 4]; 10],
    /// How many sprites have already been selected
    oams_index: usize,
    /// Whether the sprite was selected for rendering last t-cycle
    oams_used_last: bool,

    // Drawing state
    /// Background/Window FIFO and Pixel fetcher
    bgw_fifo: BackgroundFIFO,
    /// Object FIFO and Pixel fetcher
    obj_fifo: BackgroundFIFO,
    /// Whether window fetching has started this frame (y position has been reached)
    window_fetching_y: bool,
    /// Whether window fetching has started this line (x position has been reached)
    window_fetching_x: bool,
    /// The framebuffer
    framebuffer: [[u8; 160]; 144],
}

impl PPU {
    pub fn new() -> Self {
        Self {
            frame: 0,
            scanline: 0,
            dot: 0,
            tcycle: 0,
            delay: 0,
            mode: PPUState::OAMScan,
            oam_scan: [[0; 4]; 10],
            oams_index: 0,
            oams_used_last: false,
            bgw_fifo: BackgroundFIFO::new(),
            obj_fifo: BackgroundFIFO::new(),
            window_fetching_y: false,
            window_fetching_x: false,
            framebuffer: [[0; 160]; 144],
        }
    }

    pub fn get_framebuffer(&self) -> &[[u8; 160]; 144] {
        &self.framebuffer
    }

    pub fn step_mcycle(&mut self, memory: &mut Memory) {
        for _ in 0..4 {
            self.step_tcycle(memory);
        }
    }

    pub fn step_tcycle(&mut self, memory: &mut Memory) {
        if self.delay > 0 {
            assert!(
                self.mode == PPUState::Drawing,
                "Delay outside of Drawing mode is not allowed"
            );
            assert!(
                self.tcycle < 369,
                "Delay caused the Drawing to take longer than 289 tcycles"
            );
            self.delay -= 1;
            self.tcycle += 1;
            return;
        }

        self.exec_tcycle(memory);
        println!(
            "Successfully executed tcycle {} in mode {:?} (frame: {}, line: {}, dot: {})",
            self.tcycle, self.mode, self.frame, self.scanline, self.dot
        );

        self.tcycle += 1;
        if self.tcycle == 80 && self.mode == PPUState::OAMScan {
            self.set_mode(PPUState::Drawing, memory);
        } else if self.dot == 160 && self.mode == PPUState::Drawing {
            self.set_mode(PPUState::HBlank, memory);
            self.dot = 0;
        } else if self.tcycle == 456 {
            // println!("Drew line {} (frame: {})", self.scanline, self.frame);
            self.tcycle = 0;
            self.scanline += 1;

            if self.scanline <= 143 {
                self.set_mode(PPUState::OAMScan, memory);
            } else if self.scanline == 154 {
                self.set_mode(PPUState::OAMScan, memory);
                self.scanline = 0;
                self.frame += 1;
            } else {
                self.set_mode(PPUState::VBlank, memory);
            }
        }
    }

    fn exec_tcycle(&mut self, memory: &mut Memory) {
        // Extract the lcdc (LDC-Control) register
        let lcdc = LCDC::from_byte(memory.get_byte(0xFF40));

        match self.mode {
            PPUState::VBlank => {}
            PPUState::HBlank => {}
            PPUState::OAMScan => {
                if self.oams_index >= 10 {
                    // No more than 10 sprites can be rendered per scanline, so skip the rest of OAM scan
                    return;
                }

                if self.tcycle % 2 == 0 {
                    // Read first two bytes of Object Attribute
                    let b0 = memory.get_byte(0xFE00 + (self.tcycle * 2) as u16);
                    let b1 = memory.get_byte(0xFE00 + (self.tcycle * 2 + 1) as u16);

                    if b0 <= self.scanline
                        && self.scanline < b0 + if lcdc.obj_size { 16 } else { 8 }
                    {
                        // Sprite is visible on this scanline, store it for later rendering
                        self.oam_scan[self.oams_index][0] = b0; // Y position
                        self.oam_scan[self.oams_index][1] = b1; // X position
                        self.oams_used_last = true;
                    }
                } else {
                    if self.oams_used_last {
                        // Read the remaining two bytes of Object Attribute if the sprite was visible last dot
                        let b2 = memory.get_byte(0xFE00 + (self.tcycle * 2) as u16);
                        let b3 = memory.get_byte(0xFE00 + (self.tcycle * 2 + 1) as u16);

                        self.oam_scan[self.oams_index][2] = b2; // Tile index
                        self.oam_scan[self.oams_index][3] = b3; // Attributes
                        self.oams_index += 1;
                    } else {
                        // The sprite was not visible last dot, so we don't read
                        return;
                    }
                }
            }
            PPUState::Drawing => {
                // Clock FIFOs
                self.bgw_fifo.step(memory);
                self.obj_fifo.step(memory);

                if let Some(pixel) = self.bgw_fifo.take_pixel() {
                    // If obj pixel is transparent => draw bg/w pixel
                    // If bg-to-obj priority is 1 and bg/w pixel is not color 0 => draw bg/w pixel
                    // Else draw obj pixel

                    // For now only draw bg
                    self.framebuffer[self.scanline as usize][self.dot as usize] = pixel;
                    self.dot += 1;

                    // Check if window should be drawn next
                    if lcdc.window_display_enable
                        && !self.window_fetching_x
                        && self.window_fetching_y
                        && self.dot >= memory.get_byte(0xFF4B) - 7
                    {
                        self.window_fetching_x = true;
                        self.bgw_fifo.start_window();
                    }
                }
            }
        }
    }

    /// This mode sets a new PPU mode and performs all the setup. Be careful, this resets most of the PPU state and should only be called when a new mode starts
    fn set_mode(&mut self, mode: PPUState, memory: &mut Memory) {
        self.mode = mode;

        match mode {
            PPUState::OAMScan => {
                self.oams_index = 0;
                self.oams_used_last = false;
                memory.oam_access = false;
                memory.vram_access = true;
            }
            PPUState::Drawing => {
                self.dot = 0;
                memory.oam_access = false;
                memory.vram_access = false;
            }
            PPUState::HBlank => {
                memory.oam_access = true;
                memory.vram_access = true;
                memory.trigger_interrupt(1);
            }
            PPUState::VBlank => {
                memory.oam_access = true;
                memory.vram_access = true;
                memory.trigger_interrupt(0);
            }
        }
    }
}

/// Describes the current mode of the PPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PPUState {
    HBlank = 0,
    VBlank = 1,
    OAMScan = 2,
    Drawing = 3,
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
