use crate::utils::BitOps;

pub const DIV: u16 = 0xFF04;
pub const TIMA: u16 = 0xFF05;
pub const TMA: u16 = 0xFF06;
pub const TAC: u16 = 0xFF07;

const TAC_ENABLE_BIT: u8 = 2;

pub struct Timer {
    sys_clock: u16,
    tima: u8,
    tma: u8,
    tac: u8,
    overflow_pending: bool,
    overflow_delay: u8,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            sys_clock: 0xABCC,
            tima: 0,
            tma: 0,
            tac: 0xF8,
            overflow_pending: false,
            overflow_delay: 0,
        }
    }

    pub fn read_timer(&self, addr: u16) -> u8 {
        match addr {
            DIV => (self.sys_clock >> 8) as u8,
            TIMA => self.tima,
            TMA => self.tma,
            TAC => self.tac | 0xF8,
            _ => 0xFF,
        }
    }

    pub fn write_timer(&mut self, addr: u16, val: u8) {
        match addr {
            DIV => {
                let old_signal = self.timer_signal();
                self.sys_clock = 0;
                let new_signal = self.timer_signal();
                self.detect_falling_edge(old_signal, new_signal);
            }
            TIMA => {
                if self.overflow_delay == 1 {
                    // overwrite during reload cycle cancels reload
                    self.tima = val;
                    self.overflow_pending = false;
                    self.overflow_delay = 0;
                } else if self.overflow_delay != 2 {
                    self.tima = val;
                }
            }
            TMA => {
                self.tma = val;
                if self.overflow_delay == 1 {
                    self.tima = val;
                }
            }
            TAC => {
                let old_signal = self.timer_signal();
                self.tac = val & 0x07;
                let new_signal = self.timer_signal();
                self.detect_falling_edge(old_signal, new_signal);
            }
            _ => {}
        }
    }

    pub fn read_u8(&self, addr: u16) -> u8 {
        self.read_timer(addr)
    }

    pub fn write_u8(&mut self, addr: u16, val: u8) {
        self.write_timer(addr, val);
    }

    pub fn step(&mut self, cycles: u8) -> bool {
        self.tick(cycles)
    }

    pub fn tick(&mut self, cycles: u8) -> bool {
        let mut interrupt = false;

        // execute per T-cycle
        for _ in 0..(cycles as u16 * 4) {
            if self.overflow_delay > 0 {
                self.overflow_delay -= 1;
                if self.overflow_delay == 0 {
                    if self.overflow_pending {
                        self.tima = self.tma;
                        interrupt = true;
                        self.overflow_pending = false;
                    }
                }
            }

            let old_signal = self.timer_signal();
            self.sys_clock = self.sys_clock.wrapping_add(1);
            let new_signal = self.timer_signal();

            self.detect_falling_edge(old_signal, new_signal);
        }

        interrupt
    }

    fn timer_signal(&self) -> bool {
        let bit_index = match self.tac & 0x03 {
            0b00 => 9, // 4096 Hz
            0b01 => 3, // 262144 Hz
            0b10 => 5, // 65536 Hz
            0b11 => 7, // 16384 Hz
            _ => unreachable!(),
        };

        let timer_enabled = self.tac.get_bit(TAC_ENABLE_BIT);
        let bit_set = (self.sys_clock & (1 << bit_index)) != 0;
        timer_enabled && bit_set
    }

    fn detect_falling_edge(&mut self, old_signal: bool, new_signal: bool) {
        if old_signal && !new_signal {
            let (new_tima, overflow) = self.tima.overflowing_add(1);
            self.tima = new_tima;
            if overflow {
                self.overflow_pending = true;
                // 1 M-cycle (4 T-cycles) delay before TMA reload and IRQ trigger
                self.overflow_delay = 4;
            }
        }
    }
}