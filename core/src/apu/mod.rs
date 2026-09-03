pub mod channel1;
pub mod channel2;
pub mod channel3;
pub mod channel4;
pub mod length_counter;
pub mod volume_envelope;

use channel1::Channel1;
use channel2::Channel2;
use channel3::Channel3;
use channel4::Channel4;

use std::collections::VecDeque;
use crate::utils::BitOps;
use serde::{Serialize, Deserialize};

const FRAME_SEQUENCER_CYCLES: u32 = 8192; // 512 Hz divider
const SAMPLE_RATE: u32 = 44100;
const CPU_CLOCK_HZ: u32 = 4_194_304;
const CYCLES_PER_SAMPLE: f32 = CPU_CLOCK_HZ as f32 / SAMPLE_RATE as f32;

pub const NR50: u16 = 0xFF24;
pub const NR51: u16 = 0xFF25;
pub const NR52: u16 = 0xFF26;

pub const AUDIO_MASKS: [u8; 0x17] = [
    0x80, 0x3F, 0x00, 0xFF, 0xBF, // 0xFF10 - 0xFF14
    0xFF, 0x3F, 0x00, 0xFF, 0xBF, // 0xFF15 - 0xFF19
    0x7F, 0xFF, 0x9F, 0xFF, 0xBF, // 0xFF1A - 0xFF1E
    0xFF, 0xFF, 0x00, 0x00, 0xBF, // 0xFF1F - 0xFF23
    0x00, 0x00, 0x70,             // 0xFF24 - 0xFF26
];

fn default_true() -> bool {
    true
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Apu {
    pub enabled: bool,
    frame_sequencer: u8,
    frame_sequencer_timer: u32,

    pub ch1: Channel1,
    pub ch2: Channel2,
    pub ch3: Channel3,
    pub ch4: Channel4,

    nr50: u8,
    nr51: u8,

    sample_timer: f32,
    pub sample_buffer: VecDeque<f32>,

    #[serde(default = "default_true")]
    pub ch1_enabled: bool,
    #[serde(default = "default_true")]
    pub ch2_enabled: bool,
    #[serde(default = "default_true")]
    pub ch3_enabled: bool,
    #[serde(default = "default_true")]
    pub ch4_enabled: bool,
}

impl Apu {
    pub fn new() -> Self {
        Self {
            enabled: true,
            frame_sequencer: 0,
            frame_sequencer_timer: FRAME_SEQUENCER_CYCLES,

            ch1: Channel1::new(),
            ch2: Channel2::new(),
            ch3: Channel3::new(),
            ch4: Channel4::new(),

            nr50: 0x77,
            nr51: 0xF3,

            sample_timer: CYCLES_PER_SAMPLE,
            sample_buffer: VecDeque::with_capacity(4096),

            ch1_enabled: true,
            ch2_enabled: true,
            ch3_enabled: true,
            ch4_enabled: true,
        }
    }

    pub fn set_channel_enabled(&mut self, channel: usize, enabled: bool) {
        match channel {
            1 => self.ch1_enabled = enabled,
            2 => self.ch2_enabled = enabled,
            3 => self.ch3_enabled = enabled,
            4 => self.ch4_enabled = enabled,
            _ => {}
        }
    }

    pub fn is_channel_enabled(&self, channel: usize) -> bool {
        match channel {
            1 => self.ch1_enabled,
            2 => self.ch2_enabled,
            3 => self.ch3_enabled,
            4 => self.ch4_enabled,
            _ => false,
        }
    }

    pub fn update(&mut self, t_cycles: u32) {
        if !self.enabled || t_cycles == 0 {
            return;
        }

        self.ch1.step(t_cycles);
        self.ch2.step(t_cycles);
        self.ch3.step(t_cycles);
        self.ch4.step(t_cycles);

        let mut remaining = t_cycles;
        while remaining >= self.frame_sequencer_timer {
            remaining -= self.frame_sequencer_timer;
            self.frame_sequencer_timer = FRAME_SEQUENCER_CYCLES;
            self.step_frame_sequencer();
        }
        self.frame_sequencer_timer -= remaining;

        self.sample_timer -= t_cycles as f32;
        while self.sample_timer <= 0.0 {
            self.sample_timer += CYCLES_PER_SAMPLE;
            self.generate_sample();
        }
    }

    fn step_frame_sequencer(&mut self) {
        match self.frame_sequencer {
            0 => {
                self.ch1.step_length();
                self.ch2.step_length();
                self.ch3.step_length();
                self.ch4.step_length();
            }
            1 => {}
            2 => {
                self.ch1.step_length();
                self.ch2.step_length();
                self.ch3.step_length();
                self.ch4.step_length();
                self.ch1.step_sweep();
            }
            3 => {}
            4 => {
                self.ch1.step_length();
                self.ch2.step_length();
                self.ch3.step_length();
                self.ch4.step_length();
            }
            5 => {}
            6 => {
                self.ch1.step_length();
                self.ch2.step_length();
                self.ch3.step_length();
                self.ch4.step_length();
                self.ch1.step_sweep();
            }
            7 => {
                self.ch1.volume_envelope.step();
                self.ch2.volume_envelope.step();
                self.ch4.volume_envelope.step();
            }
            _ => unreachable!(),
        }
        self.frame_sequencer = (self.frame_sequencer + 1) & 7;
    }

    fn generate_sample(&mut self) {
        if !self.enabled {
            self.sample_buffer.push_back(0.0);
            self.sample_buffer.push_back(0.0);
            return;
        }

        let s1 = if self.ch1_enabled { self.ch1.get_sample() } else { 0.0 };
        let s2 = if self.ch2_enabled { self.ch2.get_sample() } else { 0.0 };
        let s3 = if self.ch3_enabled { self.ch3.get_sample() } else { 0.0 };
        let s4 = if self.ch4_enabled { self.ch4.get_sample() } else { 0.0 };

        let mut left = 0.0f32;
        let mut right = 0.0f32;

        if self.nr51.get_bit(0) { right += s1; }
        if self.nr51.get_bit(1) { right += s2; }
        if self.nr51.get_bit(2) { right += s3; }
        if self.nr51.get_bit(3) { right += s4; }

        if self.nr51.get_bit(4) { left += s1; }
        if self.nr51.get_bit(5) { left += s2; }
        if self.nr51.get_bit(6) { left += s3; }
        if self.nr51.get_bit(7) { left += s4; }

        let left_vol = ((self.nr50 >> 4) & 0x07) as f32 / 7.0;
        let right_vol = (self.nr50 & 0x07) as f32 / 7.0;

        left = (left / 4.0) * left_vol;
        right = (right / 4.0) * right_vol;

        self.sample_buffer.push_back(left);
        self.sample_buffer.push_back(right);
    }

    pub fn read_u8(&self, addr: u16) -> u8 {
        match addr {
            0xFF10..=0xFF25 => {
                let mask = AUDIO_MASKS[(addr - 0xFF10) as usize];
                if !self.enabled {
                    mask
                } else {
                    let raw = match addr {
                        0xFF10..=0xFF14 => self.ch1.read_byte(addr),
                        0xFF15 => 0x00,
                        0xFF16..=0xFF19 => self.ch2.read_byte(addr),
                        0xFF1A..=0xFF1E => self.ch3.read_byte(addr),
                        0xFF1F => 0x00,
                        0xFF20..=0xFF23 => self.ch4.read_byte(addr),
                        NR50 => self.nr50,
                        NR51 => self.nr51,
                        _ => 0x00,
                    };
                    raw | mask
                }
            }
            NR52 => {
                let mut v = if self.enabled { 0x80 } else { 0x00 };
                v |= 0x70;
                if self.enabled {
                    v.set_bit(0, self.ch1.enabled);
                    v.set_bit(1, self.ch2.enabled);
                    v.set_bit(2, self.ch3.enabled);
                    v.set_bit(3, self.ch4.enabled);
                }
                v
            }
            0xFF27..=0xFF2F => 0xFF,
            0xFF30..=0xFF3F => self.ch3.read_byte(addr),
            _ => 0xFF,
        }
    }

    pub fn write_u8(&mut self, addr: u16, val: u8) {
        let step = self.frame_sequencer;

        if addr == NR52 {
            let turn_on = (val & 0x80) != 0;
            if self.enabled && !turn_on {
                self.nr50 = 0;
                self.nr51 = 0;
                self.ch1.power_off();
                self.ch2.power_off();
                self.ch3.power_off();
                self.ch4.power_off();
                self.frame_sequencer = 0;
                self.frame_sequencer_timer = FRAME_SEQUENCER_CYCLES;
                self.enabled = false;
            } else if !self.enabled && turn_on {
                self.frame_sequencer = 0;
                self.frame_sequencer_timer = FRAME_SEQUENCER_CYCLES;
                self.enabled = true;
            }
            return;
        }

        if !self.enabled {
            match addr {
                0xFF11 => {
                    self.ch1.length_counter.load(val & 0x3F);
                }
                0xFF16 => {
                    self.ch2.length_counter.load(val & 0x3F);
                }
                0xFF1B => {
                    self.ch3.length_counter.load(val);
                }
                0xFF20 => {
                    self.ch4.length_counter.load(val & 0x3F);
                }
                0xFF30..=0xFF3F => self.ch3.write_byte(addr, val, step),
                _ => {}
            }
            return;
        }

        match addr {
            0xFF10..=0xFF14 => self.ch1.write_byte(addr, val, step),
            0xFF16..=0xFF19 => self.ch2.write_byte(addr, val, step),
            0xFF1A..=0xFF1E => self.ch3.write_byte(addr, val, step),
            0xFF20..=0xFF23 => self.ch4.write_byte(addr, val, step),
            NR50 => self.nr50 = val,
            NR51 => self.nr51 = val,
            0xFF30..=0xFF3F => self.ch3.write_byte(addr, val, step),
            _ => {}
        }
    }
}