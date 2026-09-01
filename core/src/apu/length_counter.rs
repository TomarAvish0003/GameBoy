pub struct LengthCounter {
    pub length: u16,
    pub max_length: u16,
    pub enabled: bool,
}

impl LengthCounter {
    pub fn new(max_length: u16) -> Self {
        Self {
            length: 0,
            max_length,
            enabled: false,
        }
    }

    pub fn load(&mut self, val: u8) {
        self.length = self.max_length - (val as u16 & (self.max_length - 1));
    }

    pub fn set_enabled(&mut self, enable: bool, frame_sequencer_step: u8) -> bool {
        let was_enabled = self.enabled;
        self.enabled = enable;

        // extra clocking  if current frame sequencer step is odd (1, 3, 5, 7)
        let is_odd_step = (frame_sequencer_step & 1) == 1;
        if !was_enabled && enable && is_odd_step && self.length > 0 {
            self.length -= 1;
            if self.length == 0 {
                return true;
            }
        }
        false
    }

    pub fn trigger(&mut self, frame_sequencer_step: u8) {
        if self.length == 0 {
            self.length = self.max_length;
            let is_odd_step = (frame_sequencer_step & 1) == 1;
            if self.enabled && is_odd_step {
                self.length -= 1;
            }
        }
    }

    pub fn step(&mut self) -> bool {
        if self.enabled && self.length > 0 {
            self.length -= 1;
            if self.length == 0 {
                return true;
            }
        }
        false
    }
}