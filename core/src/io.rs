use crate::timer::*;
use crate::utils::BitOps;
use serde::{Serialize, Deserialize};
use serde_big_array::BigArray;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Buttons {
    A = 0,
    B = 1,
    Select = 2,
    Start = 3,
    Right = 4,
    Left = 5,
    Up = 6,
    Down = 7,
}

const DPAD_BUTTONS: [Buttons; 4] = [
    Buttons::Right, Buttons::Left, Buttons::Up, Buttons::Down,
];

const FACE_BUTTONS: [Buttons; 4] = [
    Buttons::A, Buttons::B, Buttons::Select, Buttons::Start,
];

pub const IO_START: u16 = 0xFF00;
pub const IO_STOP: u16 = 0xFF3F;

const JOYPAD_ADDR: u16 = 0xFF00;
const IO_SIZE: usize = (IO_STOP - IO_START + 1) as usize;
const FACE_SELECT_BIT: u8 = 5;
const DPAD_SELECT_BIT: u8 = 4;

#[derive(Clone, Serialize, Deserialize)]
pub struct IO {
    buttons: [bool; 8],
    dpad_selected: bool,
    face_selected: bool,
    #[serde(with = "BigArray")]
    ram: [u8; IO_SIZE],
    timer: Timer,
}

impl IO {
    pub fn new() -> Self {
        Self {
            buttons: [false; 8],
            dpad_selected: false,
            face_selected: false,
            ram: [0; IO_SIZE],
            timer: Timer::new(),
        }
    }

    pub fn read_u8(&self, addr: u16) -> u8 {
        match addr {
            DIV..=TAC => self.timer.read_timer(addr),
            JOYPAD_ADDR => self.read_joypad(),
            IO_START..=IO_STOP => {
                let relative_addr = addr - IO_START;
                self.ram[relative_addr as usize]
            }
            _ => 0xFF,
        }
    }

    fn read_joypad(&self) -> u8 {
        let mut ret = 0xCF;

        if !self.dpad_selected {
            ret |= 1 << DPAD_SELECT_BIT;
        } else {
            ret &= !(1 << DPAD_SELECT_BIT);
        }

        if !self.face_selected {
            ret |= 1 << FACE_SELECT_BIT;
        } else {
            ret &= !(1 << FACE_SELECT_BIT);
        }

        let mut nibble = 0x0F;
        if self.dpad_selected {
            for btn in DPAD_BUTTONS {
                let idx = btn as usize;
                if self.buttons[idx] {
                    nibble &= !(1 << (idx - 4));
                }
            }
        }

        if self.face_selected {
            for btn in FACE_BUTTONS {
                let idx = btn as usize;
                if self.buttons[idx] {
                    nibble &= !(1 << idx);
                }
            }
        }

        (ret & 0xF0) | (nibble & 0x0F)
    }

    pub fn set_button(&mut self, button: Buttons, pressed: bool) {
        self.buttons[button as usize] = pressed;
    }

    pub fn update_timer(&mut self, cycles: u8) -> bool {
        self.timer.tick(cycles)
    }

    pub fn write_u8(&mut self, addr: u16, val: u8) {
        match addr {
            DIV..=TAC => {
                self.timer.write_timer(addr, val);
            }
            JOYPAD_ADDR => {
                self.face_selected = !val.get_bit(FACE_SELECT_BIT);
                self.dpad_selected = !val.get_bit(DPAD_SELECT_BIT);
            }
            IO_START..=IO_STOP => {
                let relative_addr = addr - IO_START;
                self.ram[relative_addr as usize] = val;
            }
            _ => {}
        }
    }
}