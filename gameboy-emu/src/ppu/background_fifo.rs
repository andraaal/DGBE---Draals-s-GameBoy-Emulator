use std::collections::VecDeque;

use crate::ppu::LCDC;

pub(crate) struct BackgroundFIFO {
    state: FIFOStage,
    queue: VecDeque<u8>,
    window_fetching: bool,
    pos_x: u16,
    window_line_count: u8,
    second_tcycle: bool,
}

#[expect(unused)]
impl BackgroundFIFO {
    pub fn new() -> Self {
        Self {
            state: FIFOStage::GetTile,
            queue: VecDeque::with_capacity(16),
            window_fetching: false,
            pos_x: 0,
            window_line_count: 0,
            second_tcycle: false,
        }
    }

    pub fn step(&mut self, memory: &mut crate::memory::Memory) {
        if self.second_tcycle {
            self.second_tcycle = false;
            return;
        } else {
            self.second_tcycle = true;
        }

        let lcdc = LCDC::from_byte(memory.get_byte(0xFF40));

        match self.state {
            FIFOStage::GetTile => {
                let mut tile_number = 0;

                if self.window_fetching {
                    // To select the correct row of tiles (32 tile rows long in memory and 8x8 pixels per tile)
                    tile_number += 32 * (self.window_line_count as u16 / 8);

                    // To select the correct tile in the row
                    tile_number += self.pos_x;
                } else {
                    let line_y = memory.get_byte(0xFF44) as u16;
                    let scroll_y = memory.get_byte(0xFF42) as u16;

                    // Select the correct row of tiles by adding the current scanline and the scroll offset together and using modulo to wrap around after the tile map.
                    let combined = (line_y + scroll_y) / 8;

                    // Divide by 8 to get the y offset in tiles and multiply by 32 to get the index in memory
                    tile_number = 32 * (combined % 32);

                    // Select the correct tile in the row
                    let scroll_x = memory.get_byte(0xFF43) as u16;
                    tile_number += (self.pos_x + scroll_x / 8) % 32;
                }

                self.state = FIFOStage::GetTileDataLow {
                    tile_number: tile_number as u8,
                };
            }
            FIFOStage::GetTileDataLow { tile_number } => {
                let tile_data_address = if lcdc.bg_tile_map_select {
                    0x8000 + (tile_number as u16) * 16
                } else {
                    // In this mode, the tile number is a signed byte, so we need to convert it to a signed value first.
                    let signed_tile_number = tile_number as i8;
                    (0x9000 as u16).saturating_add_signed(signed_tile_number as i16 * 16)
                };

                let low_data = memory.get_byte(tile_data_address);
                self.state = FIFOStage::GetTileDataHigh {
                    tile_number,
                    low_data,
                };
            }
            FIFOStage::GetTileDataHigh {
                tile_number,
                low_data,
            } => {
                let tile_data_address = if lcdc.bg_tile_map_select {
                    0x8000 + (tile_number as u16) * 16
                } else {
                    let signed_tile_number = tile_number as i8;
                    (0x9000 as u16).saturating_add_signed(signed_tile_number as i16 * 16)
                };

                let high_data = memory.get_byte(tile_data_address + 1);
                self.state = FIFOStage::Push {
                    low_data,
                    high_data,
                };
            }
            FIFOStage::Push {
                low_data,
                high_data,
            } => {
                if self.queue.len() == 0 {
                    for i in (0..8).rev() {
                        let low_bit = (low_data >> i) & 1;
                        let high_bit = (high_data >> i) & 1;
                        let color_id = (high_bit << 1) | low_bit;
                        self.queue.push_back(color_id);
                    }
                    self.state = FIFOStage::GetTile;
                    self.pos_x += 1;
                }
            }
        }
    }

    pub fn take_pixel(&mut self) -> Option<u8> {
        self.queue.pop_front()
    }

    pub fn start_window(&mut self) {
        self.state = FIFOStage::GetTile;
        self.queue.clear();
        self.window_fetching = true;
    }

    pub fn start_line(&mut self) {
        self.state = FIFOStage::GetTile;
        self.queue.clear();
        self.pos_x = 0;

        if self.window_fetching {
            self.window_line_count += 1;
        }
        self.window_fetching = false;
    }

    pub fn reset(&mut self) {
        self.state = FIFOStage::GetTile;
        self.queue.clear();
        self.window_fetching = false;
        self.pos_x = 0;
        self.window_line_count = 0;
    }
}

/// Describes the step the FIFO fetcher is currently in. Each step corresponds to one dot, with the last one possibly beeing executed multiple times before reseting the pixel fetcher.
enum FIFOStage {
    GetTile,
    GetTileDataLow { tile_number: u8 },
    GetTileDataHigh { tile_number: u8, low_data: u8 },
    Push { low_data: u8, high_data: u8 },
}
