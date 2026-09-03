use crate::apu::length_counter::LengthCounter;
use crate::apu::volume_envelope::VolumeEnvelope;
use crate::utils::BitOps;
use serde::{Serialize, Deserialize};

const DUTY_PATTERNS: [[bool; 8]; 4] = [
    [false, false, false, false, false, false, false, true],
    [true, false, false, false, false, false, false, true],
    [true, false, false, false, false, true, true, true],
    [false, true, true, true, true, true, true, false],
];

#[derive(Clone, Serialize, Deserialize)]
pub struct Channel2 {
    pub enabled: bool,
    dac_enabled: bool,

    pub length_counter: LengthCounter,
    pub volume_envelope: VolumeEnvelope,

    pub nr21: u8,
    pub nr22: u8,
    pub nr24: u8,

    duty_pos: usize,
    frequency: u16,
    timer: u32,
}

impl Channel2 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            dac_enabled: false,
            length_counter: LengthCounter::new(64),
            volume_envelope: VolumeEnvelope::new(),
            nr21: 0,
            nr22: 0,
            nr24: 0,
            duty_pos: 0,
            frequency: 0,
            timer: 0,
        }
    }

    pub fn power_off(&mut self) {
        self.nr21 = 0;
        self.nr22 = 0;
        self.nr24 = 0;
        self.duty_pos = 0;
        self.frequency = 0;
        self.enabled = false;
        self.dac_enabled = false;
        self.volume_envelope = VolumeEnvelope::new();
        self.length_counter.enabled = false;
    }

    pub fn step(&mut self, mut t_cycles: u32) {
        if !self.enabled {
            return;
        }

        let period = ((2048 - self.frequency) as u32) * 4;
        while t_cycles > 0 {
            if self.timer <= t_cycles {
                t_cycles -= self.timer;
                self.timer = period;
                self.duty_pos = (self.duty_pos + 1) & 7;
            } else {
                self.timer -= t_cycles;
                break;
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
        self.timer = (2048 - self.frequency) as u32 * 4;
        self.volume_envelope.trigger();
    }

    pub fn get_sample(&self) -> f32 {
        if !self.enabled || !self.dac_enabled || self.volume_envelope.current_volume == 0 {
            return 0.0;
        }

        let duty = (self.nr21 >> 6) as usize;
        let is_high = DUTY_PATTERNS[duty][self.duty_pos];
        let sample = if is_high {
            self.volume_envelope.current_volume
        } else {
            0
        };

        (sample as f32 / 7.5) - 1.0
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            0xFF16 => self.nr21 & 0xC0, // Only bits 6..7 readable
            0xFF17 => self.nr22,
            0xFF18 => 0x00,
            0xFF19 => self.nr24 & 0x40, // Only bit 6 readable
            _ => 0x00,
        }
    }

    pub fn write_byte(&mut self, addr: u16, val: u8, frame_sequencer_step: u8) {
        match addr {
            0xFF16 => {
                self.nr21 = val;
                self.length_counter.load(val & 0x3F);
            }
            0xFF17 => {
                self.nr22 = val;
                self.volume_envelope.write_byte(val);
                self.dac_enabled = (val & 0xF8) != 0;
                if !self.dac_enabled {
                    self.enabled = false;
                }
            }
            0xFF18 => {
                self.frequency = (self.frequency & 0x0700) | (val as u16);
            }
            0xFF19 => {
                self.nr24 = val;
                self.frequency = (self.frequency & 0x00FF) | (((val & 0x07) as u16) << 8);
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