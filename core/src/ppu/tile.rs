use crate::utils::BitOps;

#[derive(Clone, Copy)]
pub struct Tile {
    pub pixel: [[u8; 8]; 8],
}

impl Tile {
    pub fn new() -> Self {
        Self {
            pixel: [[0; 8]; 8],
        }
    }

    pub fn read_u8(&self, offset: u16) -> u8 {
        let row = (offset / 2) as usize;
        let bit = if offset % 2 == 0 { 0 } else { 1 };
        let mut ret = 0u8;

        for i in 0..8 {
            let bit_val = if self.pixel[row][i].get_bit(bit) { 1 } else { 0 };
            ret |= bit_val << (7 - i);
        }
        ret
    }

    pub fn write_u8(&mut self, offset: u16, val: u8) {
        let row = (offset / 2) as usize;
        let bit = if offset % 2 == 0 { 0 } else { 1 };

        for i in 0..8 {
            self.pixel[row][7 - i].set_bit(bit, val.get_bit(i as u8));
        }
    }

    pub fn get_row(&self, row: usize) -> [u8; 8] {
        self.pixel[row]
    }
}