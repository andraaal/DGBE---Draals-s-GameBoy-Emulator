use std::collections::VecDeque;

use crate::ppu::LCDC;

pub(crate) struct BackgroundFIFO {
    state: FIFOStage,
    queue: VecDeque<u8>,
    window_fetching: bool,
    pos_x: u8,
}

impl BackgroundFIFO {
    pub fn new() -> Self {
        Self {
            state: FIFOStage::GetTile0,
            queue: VecDeque::with_capacity(16),
            window_fetching: false,
            pos_x: 0,
        }
    }

    pub fn step(&mut self, memory: &mut crate::memory::Memory) {
        let lcdc = LCDC::from_byte(memory.read_byte(0xFF40));

        match self.state {
            FIFOStage::GetTile0 => {
                let tile_number;
                match self.window_fetching {
                    false => {
                        // Tile number
                        tile_number = ((memory.get_byte(0xFF43) / 8) & 0x1F)
                            + self.pos_x
                            + 32 * ((memory.get_byte(0xFF44) + memory.get_byte(0xFF42)) / 8);
                    }
                    true => {
                        // Fetch tile number from window tile map
                        tile_number = 0;
                    }
                }
                println!("{}", tile_number);
            }
            FIFOStage::GetTile1 => {}
            FIFOStage::GetTileDataLow0 => {}
            FIFOStage::GetTileDataLow1 => {}
            FIFOStage::GetTileDataHigh0 => {}
            FIFOStage::GetTileDataHigh1 => {}
            FIFOStage::Sleep0 => {}
            FIFOStage::Sleep1 => {}
            FIFOStage::Push => {}
        }
        self.state.next();
    }

    pub fn restart(&mut self, window_fetching: bool) {
        self.state = FIFOStage::GetTile0;
        self.queue.clear();
        self.window_fetching = window_fetching;
    }
}

/// Describes the step the FIFO fetcher is currently in. Each step corresponds to one dot, with the last one possibly beeing executed multiple times.
enum FIFOStage {
    GetTile0,
    GetTile1,
    GetTileDataLow0,
    GetTileDataLow1,
    GetTileDataHigh0,
    GetTileDataHigh1,
    Sleep0,
    Sleep1,
    Push,
}

impl FIFOStage {
    fn next(&mut self) {
        *self = match self {
            FIFOStage::GetTile0 => FIFOStage::GetTile1,
            FIFOStage::GetTile1 => FIFOStage::GetTileDataLow0,
            FIFOStage::GetTileDataLow0 => FIFOStage::GetTileDataLow1,
            FIFOStage::GetTileDataLow1 => FIFOStage::GetTileDataHigh0,
            FIFOStage::GetTileDataHigh0 => FIFOStage::GetTileDataHigh1,
            FIFOStage::GetTileDataHigh1 => FIFOStage::Sleep0,
            FIFOStage::Sleep0 => FIFOStage::Sleep1,
            FIFOStage::Sleep1 => FIFOStage::Push,
            FIFOStage::Push => FIFOStage::GetTile0,
        }
    }
}
