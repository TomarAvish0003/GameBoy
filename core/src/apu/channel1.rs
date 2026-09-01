use crate::apu::length_counter::LengthCounter;
use crate::apu::volume_envelope::VolumeEnvelope;
use crate::utils::BitOps;

const DUTY_PATTERNS: [[bool; 8]; 4] = [
    [false, false, false, false, false, false, false, true],
    [true, false, false, false, false, false, false, true],
    [true, false, false, false, false, true, true, true],
    [false, true, true, true, true, true, true, false],
];

pub struct Channel1 {
    pub enabled: bool,
    dac_enabled: bool,

    pub length_counter: LengthCounter,
    pub volume_envelope: VolumeEnvelope,

    pub nr10: u8,
    pub nr11: u8,
    pub nr12: u8,
    pub nr14: u8,

    duty_pos: usize,
    frequency: u16,
    timer: u32,

    sweep_timer: u8,
    sweep_shadow_freq: u16,
    sweep_enabled: bool,
    sweep_calculated_negate: bool,
}

impl Channel1 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            dac_enabled: false,
            length_counter: LengthCounter::new(64),
            volume_envelope: VolumeEnvelope::new(),
            nr10: 0,
            nr11: 0,
            nr12: 0,
            nr14: 0,
            duty_pos: 0,
            frequency: 0,
            timer: 0,
            sweep_timer: 0,
            sweep_shadow_freq: 0,
            sweep_enabled: false,
            sweep_calculated_negate: false,
        }
    }

    pub fn power_off(&mut self) {
        self.nr10 = 0;
        self.nr11 = 0;
        self.nr12 = 0;
        self.nr14 = 0;
        self.duty_pos = 0;
        self.frequency = 0;
        self.enabled = false;
        self.dac_enabled = false;
        self.sweep_enabled = false;
        self.sweep_calculated_negate = false;
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

    pub fn step_sweep(&mut self) {
        if self.sweep_timer > 0 {
            self.sweep_timer -= 1;
        }

        let period = (self.nr10 >> 4) & 0x07;
        let shift = self.nr10 & 0x07;

        if self.sweep_timer == 0 {
            self.sweep_timer = if period == 0 { 8 } else { period };

            if self.sweep_enabled && period > 0 {
                let new_freq = self.calculate_sweep_freq();
                if new_freq <= 2047 && shift > 0 {
                    self.frequency = new_freq;
                    self.sweep_shadow_freq = new_freq;
                    if self.calculate_sweep_freq() > 2047 {
                        self.enabled = false;
                    }
                } else if new_freq > 2047 {
                    self.enabled = false;
                }
            }
        }
    }

    fn calculate_sweep_freq(&mut self) -> u16 {
        let shift = self.nr10 & 0x07;
        let sub = self.nr10.get_bit(3);
        if sub {
            self.sweep_calculated_negate = true;
        }
        let delta = self.sweep_shadow_freq >> shift;
        if sub {
            self.sweep_shadow_freq.saturating_sub(delta)
        } else {
            self.sweep_shadow_freq.saturating_add(delta)
        }
    }

    pub fn trigger(&mut self, frame_sequencer_step: u8) {
        self.enabled = self.dac_enabled;
        self.length_counter.trigger(frame_sequencer_step);
        self.timer = (2048 - self.frequency) as u32 * 4;
        self.volume_envelope.trigger();

        let period = (self.nr10 >> 4) & 0x07;
        let shift = self.nr10 & 0x07;
        self.sweep_shadow_freq = self.frequency;
        self.sweep_timer = if period == 0 { 8 } else { period };
        self.sweep_enabled = period > 0 || shift > 0;
        self.sweep_calculated_negate = false;
        if shift > 0 && self.calculate_sweep_freq() > 2047 {
            self.enabled = false;
        }
    }

    pub fn get_sample(&self) -> f32 {
        if !self.enabled || !self.dac_enabled || self.volume_envelope.current_volume == 0 {
            return 0.0;
        }

        let duty = (self.nr11 >> 6) as usize;
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
            0xFF10 => self.nr10,
            0xFF11 => self.nr11 & 0xC0, // Only bits 6..7 readable
            0xFF12 => self.nr12,
            0xFF13 => 0x00,
            0xFF14 => self.nr14 & 0x40, // Only bit 6 readable
            _ => 0x00,
        }
    }

    pub fn write_byte(&mut self, addr: u16, val: u8, frame_sequencer_step: u8) {
        match addr {
            0xFF10 => {
                let old_negate = self.nr10.get_bit(3);
                let new_negate = val.get_bit(3);
                if old_negate && !new_negate && self.sweep_calculated_negate {
                    self.enabled = false;
                }
                self.nr10 = val;
            }
            0xFF11 => {
                self.nr11 = val;
                self.length_counter.load(val & 0x3F);
            }
            0xFF12 => {
                self.nr12 = val;
                self.volume_envelope.write_byte(val);
                self.dac_enabled = (val & 0xF8) != 0;
                if !self.dac_enabled {
                    self.enabled = false;
                }
            }
            0xFF13 => {
                self.frequency = (self.frequency & 0x0700) | (val as u16);
            }
            0xFF14 => {
                self.nr14 = val;
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