pub struct VolumeEnvelope {
    pub initial_volume: u8,
    pub current_volume: u8,
    pub direction_add: bool,
    pub period: u8,
    pub timer: u8,
    pub running: bool,
}

impl VolumeEnvelope {
    pub fn new() -> Self {
        Self {
            initial_volume: 0,
            current_volume: 0,
            direction_add: false,
            period: 0,
            timer: 0,
            running: false,
        }
    }

    pub fn write_byte(&mut self, val: u8) {
        self.initial_volume = val >> 4;
        self.direction_add = (val & 0x08) != 0;
        self.period = val & 0x07;
    }

    pub fn trigger(&mut self) {
        self.current_volume = self.initial_volume;
        self.timer = if self.period == 0 { 8 } else { self.period };
        self.running = true;
    }

    pub fn step(&mut self) {
        if self.period == 0 || !self.running {
            return;
        }

        if self.timer > 0 {
            self.timer -= 1;
        }

        if self.timer == 0 {
            self.timer = if self.period == 0 { 8 } else { self.period };
            if self.direction_add {
                if self.current_volume < 15 {
                    self.current_volume += 1;
                } else {
                    self.running = false;
                }
            } else {
                if self.current_volume > 0 {
                    self.current_volume -= 1;
                } else {
                    self.running = false;
                }
            }
        }
    }
}