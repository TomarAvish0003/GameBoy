use crate::apu::length_counter::LengthCounter;
use crate::utils::BitOps;
use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Channel3 {
    pub enabled: bool,
    pub dac_enabled: bool,

    pub length_counter: LengthCounter,

    pub nr30: u8,
    pub nr32: u8,
    pub nr34: u8,

    pub frequency: u16,
    pub timer: u32,
    pub position: usize,

    pub wave_ram: [u8; 16],
    pub just_read: bool,
}

impl Channel3 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            dac_enabled: false,
            length_counter: LengthCounter::new(256),
            nr30: 0,
            nr32: 0,
            nr34: 0,
            frequency: 0,
            timer: 0,
            position: 0,
            wave_ram: [0; 16],
            just_read: false,
        }
    }

    pub fn power_off(&mut self) {
        self.nr30 = 0;
        self.nr32 = 0;
        self.nr34 = 0;
        self.frequency = 0;
        self.enabled = false;
        self.dac_enabled = false;
        self.position = 0;
        self.length_counter.enabled = false;
        self.just_read = false;
    }

    #[inline]
    fn get_period(&self) -> u32 {
        (2048 - self.frequency as u32) * 2
    }

    pub fn step(&mut self, mut t_cycles: u32) {
        if !self.enabled || !self.dac_enabled {
            self.just_read = false;
            return;
        }

        self.just_read = false;
        while t_cycles > 0 {
            if self.timer <= t_cycles {
                t_cycles -= self.timer;
                self.timer = self.get_period();
                self.position = (self.position + 1) & 31;
                self.just_read = true;
            } else {
                self.timer -= t_cycles;
                t_cycles = 0;
                self.just_read = false;
            }
        }
    }

    pub fn step_length(&mut self) {
        if self.length_counter.step() {
            self.enabled = false;
        }
    }

    pub fn is_accessing_wave_ram(&self) -> bool {
        self.just_read
    }

    pub fn trigger(&mut self, frame_sequencer_step: u8) {
        let was_enabled = self.enabled && self.dac_enabled;
        if was_enabled && self.timer <= 2 {
            let offset = ((self.position + 1) / 2) & 0x0F;
            if offset < 4 {
                self.wave_ram[0] = self.wave_ram[offset as usize];
            } else {
                let block = (offset & !3) as usize;
                let b0 = self.wave_ram[block];
                let b1 = self.wave_ram[block + 1];
                let b2 = self.wave_ram[block + 2];
                let b3 = self.wave_ram[block + 3];
                self.wave_ram[0] = b0;
                self.wave_ram[1] = b1;
                self.wave_ram[2] = b2;
                self.wave_ram[3] = b3;
            }
        }

        self.enabled = self.dac_enabled;
        self.length_counter.trigger(frame_sequencer_step);
        self.timer = self.get_period() + 6;
        self.position = 0;
        self.just_read = false;
    }

    pub fn get_sample(&self) -> f32 {
        if !self.enabled || !self.dac_enabled {
            return 0.0;
        }

        let byte = self.wave_ram[self.position / 2];
        let sample = if self.position % 2 == 0 {
            byte >> 4
        } else {
            byte & 0x0F
        };

        let volume_code = (self.nr32 >> 5) & 0x03;
        let shifted = match volume_code {
            0 => 0,
            1 => sample,
            2 => sample >> 1,
            3 => sample >> 2,
            _ => 0,
        };

        (shifted as f32 / 7.5) - 1.0
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            0xFF1A => self.nr30 & 0x80, // Only bit 7 readable
            0xFF1B => 0x00,
            0xFF1C => self.nr32 & 0x60, // Only bits 5..6 readable
            0xFF1D => 0x00,
            0xFF1E => self.nr34 & 0x40, // Only bit 6 readable
            0xFF30..=0xFF3F => {
                if self.enabled && self.dac_enabled {
                    if self.is_accessing_wave_ram() {
                        self.wave_ram[self.position / 2]
                    } else {
                        0xFF
                    }
                } else {
                    self.wave_ram[(addr - 0xFF30) as usize]
                }
            }
            _ => 0x00,
        }
    }

    pub fn write_byte(&mut self, addr: u16, val: u8, frame_sequencer_step: u8) {
        match addr {
            0xFF1A => {
                self.nr30 = val;
                self.dac_enabled = val.get_bit(7);
                if !self.dac_enabled {
                    self.enabled = false;
                }
            }
            0xFF1B => self.length_counter.load(val),
            0xFF1C => self.nr32 = val,
            0xFF1D => {
                self.frequency = (self.frequency & 0x0700) | (val as u16);
            }
            0xFF1E => {
                self.nr34 = val;
                self.frequency = (self.frequency & 0x00FF) | (((val & 0x07) as u16) << 8);
                if self.length_counter.set_enabled(val.get_bit(6), frame_sequencer_step) {
                    self.enabled = false;
                }
                if val.get_bit(7) {
                    self.trigger(frame_sequencer_step);
                }
            }
            0xFF30..=0xFF3F => {
                if self.enabled && self.dac_enabled {
                    if self.is_accessing_wave_ram() {
                        self.wave_ram[self.position / 2] = val;
                    }
                } else {
                    self.wave_ram[(addr - 0xFF30) as usize] = val;
                }
            }
            _ => {}
        }
    }
}