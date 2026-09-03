use crate::apu::length_counter::LengthCounter;
use crate::apu::volume_envelope::VolumeEnvelope;
use crate::utils::BitOps;
use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Channel4 {
    pub enabled: bool,
    dac_enabled: bool,

    pub length_counter: LengthCounter,
    pub volume_envelope: VolumeEnvelope,

    pub nr41: u8,
    pub nr42: u8,
    pub nr43: u8,
    pub nr44: u8,

    lfsr: u16,
    timer: u32,
}

impl Channel4 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            dac_enabled: false,
            length_counter: LengthCounter::new(64),
            volume_envelope: VolumeEnvelope::new(),
            nr41: 0,
            nr42: 0,
            nr43: 0,
            nr44: 0,
            lfsr: 0x7FFF,
            timer: 8,
        }
    }

    pub fn power_off(&mut self) {
        self.nr41 = 0;
        self.nr42 = 0;
        self.nr43 = 0;
        self.nr44 = 0;
        self.enabled = false;
        self.dac_enabled = false;
        self.volume_envelope = VolumeEnvelope::new();
        self.length_counter.enabled = false;
    }

    fn get_divisor(&self) -> u32 {
        let divisor_code = self.nr43 & 0x07;
        let clock_shift = self.nr43 >> 4;
        let base = match divisor_code {
            0 => 8,
            n => (n as u32) * 16,
        };
        (base << clock_shift).max(1)
    }

    pub fn step(&mut self, mut t_cycles: u32) {
        while t_cycles > 0 {
            if self.timer <= t_cycles {
                t_cycles -= self.timer;
                self.timer = self.get_divisor();

                let bit0 = self.lfsr & 1;
                let bit1 = (self.lfsr >> 1) & 1;
                let result = bit0 ^ bit1;

                self.lfsr = (self.lfsr >> 1) | (result << 14);
                if self.nr43.get_bit(3) {
                    self.lfsr = (self.lfsr & !0x40) | (result << 6);
                }
            } else {
                self.timer -= t_cycles;
                t_cycles = 0;
            }
        }
    }

    pub fn step_length(&mut self) {
        if self.length_counter.step() {
            self.enabled = false;
        }
    }

    pub fn trigger(&mut self, frame_sequencer_step: u8) {
        self.enabled = self.dac_enabled;
        self.length_counter.trigger(frame_sequencer_step);
        self.timer = self.get_divisor();
        self.lfsr = 0x7FFF;
        self.volume_envelope.trigger();
    }

    pub fn get_sample(&self) -> f32 {
        if !self.enabled || !self.dac_enabled || self.volume_envelope.current_volume == 0 {
            return 0.0;
        }

        let sample = if (self.lfsr & 1) == 0 {
            self.volume_envelope.current_volume
        } else {
            0
        };

        (sample as f32 / 7.5) - 1.0
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            0xFF20 => 0x00,             // NR41 is completely write-only
            0xFF21 => self.nr42,
            0xFF22 => self.nr43,
            0xFF23 => self.nr44 & 0x40, // Only bit 6 readable
            _ => 0x00,
        }
    }

    pub fn write_byte(&mut self, addr: u16, val: u8, frame_sequencer_step: u8) {
        match addr {
            0xFF20 => {
                self.nr41 = val;
                self.length_counter.load(val & 0x3F);
            }
            0xFF21 => {
                self.nr42 = val;
                self.volume_envelope.write_byte(val);
                self.dac_enabled = (val & 0xF8) != 0;
                if !self.dac_enabled {
                    self.enabled = false;
                }
            }
            0xFF22 => {
                self.nr43 = val;
            }
            0xFF23 => {
                self.nr44 = val;
                if self.length_counter.set_enabled(val.get_bit(6), frame_sequencer_step) {
                    self.enabled = false;
                }
                if val.get_bit(7) {
                    self.trigger(frame_sequencer_step);
                }
            }
            _ => {}
        }
    }
}