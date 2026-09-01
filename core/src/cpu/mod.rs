use crate::bus::Bus;
pub mod opcodes;
use crate::utils::*;
use crate::ppu::modes::LcdResults;
use crate::io::Buttons;

const IF: u16 = 0xFF0F;
const IE: u16 = 0xFFFF;

const IRQ_PRIORITIES: [Interrupts; 5] = [
    Interrupts::Vblank,
    Interrupts::Stat,
    Interrupts::Timer,
    Interrupts::Serial,
    Interrupts::Joypad,
];

#[derive(Copy, Clone)]
pub enum Interrupts {
    Vblank,
    Stat,
    Timer,
    Serial,
    Joypad,
}

pub struct Cpu {
    pub pc: u16,
    pub sp: u16,
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub f: u8,
    pub h: u8,
    pub l: u8,
    pub irq_enabled: bool,
    pub ime_scheduled: bool,
    pub halted: bool,
    pub halt_bug: bool,
    pub bus: Bus,
    pub last_read: Option<u16>,
    pub last_write: Option<u16>,
    pub dirty_battery: bool,
    pub pending_draw: bool,
}

impl Interrupts {
    pub fn get_vector(&self) -> u16 {
        match *self {
            Interrupts::Vblank => 0x0040,
            Interrupts::Stat => 0x0048,
            Interrupts::Timer => 0x0050,
            Interrupts::Serial => 0x0058,
            Interrupts::Joypad => 0x0060,
        }
    }
}

impl Cpu {
    pub fn new() -> Self {
        let mut cpu = Self {
            pc: 0x0100,
            sp: 0xFFFE,
            a: 0x01,
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            f: 0xB0,
            h: 0x01,
            l: 0x4D,
            irq_enabled: false,
            ime_scheduled: false,
            halted: false,
            halt_bug: false,
            bus: Bus::new(),
            last_read: None,
            last_write: None,
            dirty_battery: false,
            pending_draw: false,
        };

        cpu.write_ram(0xFF10, 0x80);
        cpu.write_ram(0xFF11, 0xBF);
        cpu.write_ram(0xFF12, 0xF3);
        cpu.write_ram(0xFF14, 0xBF);
        cpu.write_ram(0xFF16, 0x3F);
        cpu.write_ram(0xFF19, 0xBF);
        cpu.write_ram(0xFF1A, 0x7F);
        cpu.write_ram(0xFF1B, 0xFF);
        cpu.write_ram(0xFF1C, 0x9F);
        cpu.write_ram(0xFF1E, 0xBF);
        cpu.write_ram(0xFF20, 0xFF);
        cpu.write_ram(0xFF23, 0xBF);
        cpu.write_ram(0xFF24, 0x77);
        cpu.write_ram(0xFF25, 0xF3);
        cpu.write_ram(0xFF26, 0xF1);
        cpu.write_ram(0xFF40, 0x91);
        cpu.write_ram(0xFF47, 0xFC);
        cpu.write_ram(0xFF48, 0xFF);
        cpu.write_ram(0xFF49, 0xFF);

        cpu
    }

    #[inline]
    pub fn advance_t_cycles(&mut self, t_cycles: u32) {
        let m_cycles = (t_cycles / 4) as u8;
        if m_cycles > 0 {
            let ppu_result = self.bus.update_ppu(m_cycles);
            if ppu_result.irq {
                self.enable_irq_type(Interrupts::Stat, true);
            }
            if let LcdResults::RenderFrame = ppu_result.lcd_result {
                self.bus.render_scanline();
                self.enable_irq_type(Interrupts::Vblank, true);
                self.pending_draw = true;
            }
            if self.bus.update_timer(m_cycles) {
                self.enable_irq_type(Interrupts::Timer, true);
            }
        }
        // advance apu by exact T-cycles
        self.bus.update_apu(t_cycles);
    }

    #[inline]
    pub fn mem_read(&mut self, addr: u16) -> u8 {
        self.advance_t_cycles(2);
        self.last_read = Some(addr);
        let val = self.bus.read_ram(addr);
        self.advance_t_cycles(2);
        val
    }

    #[inline]
    pub fn mem_write(&mut self, addr: u16, val: u8) {
        self.advance_t_cycles(2);
        self.last_write = Some(addr);
        self.dirty_battery |= self.bus.write_ram(addr, val);
        self.advance_t_cycles(2);
    }

    #[inline]
    pub fn internal_delay(&mut self) {
        self.advance_t_cycles(4);
    }

    pub fn clean_battery(&mut self) {
        self.dirty_battery = false;
    }

    pub fn is_battery_dirty(&self) -> bool {
        self.dirty_battery
    }

    pub fn has_battery(&self) -> bool {
        self.bus.has_battery()
    }

    pub fn get_title(&self) -> String {
        self.bus.get_title()
    }

    pub fn get_read(&self) -> Option<u16> {
        self.last_read
    }

    pub fn get_write(&self) -> Option<u16> {
        self.last_write
    }

    pub fn read_ram(&self, addr: u16) -> u8 {
        self.bus.read_ram(addr)
    }

    pub fn write_ram(&mut self, addr: u16, val: u8) {
        self.last_write = Some(addr);
        self.dirty_battery |= self.bus.write_ram(addr, val);
    }

    pub fn get_battery_data(&self) -> &[u8] {
        self.bus.get_battery_data()
    }

    pub fn set_battery_data(&mut self, data: &[u8]) {
        self.bus.set_battery_data(data);
    }

    pub fn get_r8(&mut self, r: Regs) -> u8 {
        match r {
            Regs::A => self.a,
            Regs::B => self.b,
            Regs::C => self.c,
            Regs::D => self.d,
            Regs::E => self.e,
            Regs::F => self.f,
            Regs::H => self.h,
            Regs::L => self.l,
            Regs::HL => {
                let addr = self.get_r16(Regs16::HL);
                self.mem_read(addr)
            }
        }
    }

    pub fn set_r8(&mut self, r: Regs, val: u8) {
        match r {
            Regs::A => self.a = val,
            Regs::B => self.b = val,
            Regs::C => self.c = val,
            Regs::D => self.d = val,
            Regs::E => self.e = val,
            Regs::F => self.f = val & 0xF0,
            Regs::H => self.h = val,
            Regs::L => self.l = val,
            Regs::HL => {
                let addr = self.get_r16(Regs16::HL);
                self.mem_write(addr, val);
            }
        }
    }

    pub fn get_r16(&self, r: Regs16) -> u16 {
        match r {
            Regs16::AF => merge_bytes(self.a, self.f),
            Regs16::BC => merge_bytes(self.b, self.c),
            Regs16::DE => merge_bytes(self.d, self.e),
            Regs16::HL => merge_bytes(self.h, self.l),
            Regs16::SP => self.sp,
        }
    }

    pub fn set_r16(&mut self, r: Regs16, val: u16) {
        let high = val.high_byte();
        let low = val.low_byte();
        match r {
            Regs16::AF => {
                self.a = high;
                self.f = low & 0xF0;
            }
            Regs16::BC => {
                self.b = high;
                self.c = low;
            }
            Regs16::DE => {
                self.d = high;
                self.e = low;
            }
            Regs16::HL => {
                self.h = high;
                self.l = low;
            }
            Regs16::SP => self.sp = val,
        }
    }

    pub fn get_flag(&self, f: Flags) -> bool {
        match f {
            Flags::Z => (self.f & 0b1000_0000) != 0,
            Flags::N => (self.f & 0b0100_0000) != 0,
            Flags::H => (self.f & 0b0010_0000) != 0,
            Flags::C => (self.f & 0b0001_0000) != 0,
        }
    }

    pub fn set_flag(&mut self, f: Flags, val: bool) {
        if val {
            match f {
                Flags::Z => self.f |= 0b1000_0000,
                Flags::N => self.f |= 0b0100_0000,
                Flags::H => self.f |= 0b0010_0000,
                Flags::C => self.f |= 0b0001_0000,
            }
        } else {
            match f {
                Flags::Z => self.f &= !0b1000_0000,
                Flags::N => self.f &= !0b0100_0000,
                Flags::H => self.f &= !0b0010_0000,
                Flags::C => self.f &= !0b0001_0000,
            }
        }
    }

    pub fn fetch(&mut self) -> u8 {
        let val = self.mem_read(self.pc);
        if self.halt_bug {
            self.halt_bug = false;
        } else {
            self.pc = self.pc.wrapping_add(1);
        }
        val
    }

    pub fn fetch_u16(&mut self) -> u16 {
        let low = self.fetch();
        let high = self.fetch();
        merge_bytes(high, low)
    }

    pub fn dec_r16(&mut self, r: Regs16) {
        let val = self.get_r16(r);
        self.set_r16(r, val.wrapping_sub(1));
    }

    pub fn inc_r16(&mut self, r: Regs16) {
        let val = self.get_r16(r);
        self.set_r16(r, val.wrapping_add(1));
    }

    pub fn dec_r8(&mut self, r: Regs) {
        let val = self.get_r8(r);
        let dec = val.wrapping_sub(1);
        let set_h = check_h_borrow_u8(val, 1);

        self.set_r8(r, dec);
        self.set_flag(Flags::N, true);
        self.set_flag(Flags::Z, dec == 0);
        self.set_flag(Flags::H, set_h);
    }

    pub fn inc_r8(&mut self, r: Regs) {
        let val = self.get_r8(r);
        let inc = val.wrapping_add(1);
        let set_h = check_h_carry_u8(val, 1);

        self.set_r8(r, inc);
        self.set_flag(Flags::N, false);
        self.set_flag(Flags::Z, inc == 0);
        self.set_flag(Flags::H, set_h);
    }

    pub fn and_a_u8(&mut self, val: u8) {
        let mut a = self.get_r8(Regs::A);
        a &= val;

        self.set_r8(Regs::A, a);
        self.set_flag(Flags::Z, a == 0);
        self.set_flag(Flags::N, false);
        self.set_flag(Flags::H, true);
        self.set_flag(Flags::C, false);
    }

    pub fn or_a_u8(&mut self, val: u8) {
        let mut a = self.get_r8(Regs::A);
        a |= val;

        self.set_r8(Regs::A, a);
        self.set_flag(Flags::Z, a == 0);
        self.set_flag(Flags::N, false);
        self.set_flag(Flags::H, false);
        self.set_flag(Flags::C, false);
    }

    pub fn xor_a_u8(&mut self, val: u8) {
        let mut a = self.get_r8(Regs::A);
        a ^= val;

        self.set_r8(Regs::A, a);
        self.set_flag(Flags::Z, a == 0);
        self.set_flag(Flags::N, false);
        self.set_flag(Flags::H, false);
        self.set_flag(Flags::C, false);
    }

    pub fn add_a_u8(&mut self, val: u8, adc: bool) {
        let carry: u8 = if adc && self.get_flag(Flags::C) { 1 } else { 0 };
        let a = self.get_r8(Regs::A);
        let result1 = a.overflowing_add(val);
        let h_check1 = check_h_carry_u8(a, val);
        let result2 = result1.0.overflowing_add(carry);
        let h_check2 = check_h_carry_u8(result1.0, carry);

        let set_h = h_check1 || h_check2;
        let set_c = result1.1 || result2.1;

        self.set_flag(Flags::N, false);
        self.set_flag(Flags::C, set_c);
        self.set_flag(Flags::H, set_h);
        self.set_flag(Flags::Z, result2.0 == 0);
        self.set_r8(Regs::A, result2.0);
    }

    pub fn sub_a_u8(&mut self, val: u8, sbc: bool) {
        let carry: u8 = if sbc && self.get_flag(Flags::C) { 1 } else { 0 };
        let a = self.get_r8(Regs::A);
        let result1 = a.overflowing_sub(val);
        let check_h1 = check_h_borrow_u8(a, val);
        let result2 = result1.0.overflowing_sub(carry);
        let check_h2 = check_h_borrow_u8(result1.0, carry);

        let set_h = check_h1 || check_h2;

        self.set_flag(Flags::N, true);
        self.set_flag(Flags::Z, result2.0 == 0);
        self.set_flag(Flags::H, set_h);
        self.set_flag(Flags::C, result1.1 || result2.1);
        self.set_r8(Regs::A, result2.0);
    }

    pub fn cp_a_u8(&mut self, val: u8) {
        let a = self.get_r8(Regs::A);
        let set_h = check_h_borrow_u8(a, val);

        self.set_flag(Flags::Z, a == val);
        self.set_flag(Flags::N, true);
        self.set_flag(Flags::H, set_h);
        self.set_flag(Flags::C, a < val);
    }

    pub fn add_r16(&mut self, dst_r: Regs16, src_r: Regs16) {
        let dst = self.get_r16(dst_r);
        let src = self.get_r16(src_r);
        let res = dst.overflowing_add(src);
        let set_h = check_h_carry_u16(dst, src);

        self.set_r16(dst_r, res.0);
        self.set_flag(Flags::N, false);
        self.set_flag(Flags::H, set_h);
        self.set_flag(Flags::C, res.1);
    }

    pub fn pop(&mut self) -> u16 {
        let low = self.mem_read(self.sp);
        self.sp = self.sp.wrapping_add(1);
        let high = self.mem_read(self.sp);
        self.sp = self.sp.wrapping_add(1);
        merge_bytes(high, low)
    }

    pub fn push(&mut self, val: u16) {
        self.internal_delay();
        self.sp = self.sp.wrapping_sub(1);
        self.mem_write(self.sp, (val >> 8) as u8);
        self.sp = self.sp.wrapping_sub(1);
        self.mem_write(self.sp, (val & 0xFF) as u8);
    }

    pub fn get_pc(&self) -> u16 {
        self.pc
    }

    pub fn set_pc(&mut self, val: u16) {
        self.pc = val;
    }

    pub fn rotate_left(&mut self, reg: Regs, through_carry: bool) {
        let val = self.get_r8(reg);
        let msb = val.get_bit(7);
        let mut new = val.rotate_left(1);
        if through_carry {
            new.set_bit(0, self.get_flag(Flags::C));
        }
        self.set_r8(reg, new);
        self.set_flag(Flags::Z, new == 0);
        self.set_flag(Flags::N, false);
        self.set_flag(Flags::H, false);
        self.set_flag(Flags::C, msb);
    }

    pub fn rotate_right(&mut self, reg: Regs, through_carry: bool) {
        let val = self.get_r8(reg);
        let lsb = val.get_bit(0);
        let mut new = val.rotate_right(1);
        if through_carry {
            new.set_bit(7, self.get_flag(Flags::C));
        }
        self.set_r8(reg, new);
        self.set_flag(Flags::Z, new == 0);
        self.set_flag(Flags::N, false);
        self.set_flag(Flags::H, false);
        self.set_flag(Flags::C, lsb);
    }

    pub fn shift_left(&mut self, reg: Regs) {
        let val = self.get_r8(reg);
        let msb = val.get_bit(7);
        let res = val.wrapping_shl(1);

        self.set_r8(reg, res);
        self.set_flag(Flags::Z, res == 0);
        self.set_flag(Flags::N, false);
        self.set_flag(Flags::H, false);
        self.set_flag(Flags::C, msb);
    }

    pub fn shift_right(&mut self, reg: Regs, arith: bool) {
        let val = self.get_r8(reg);
        let lsb = val.get_bit(0);
        let msb = val.get_bit(7);
        let mut res = val.wrapping_shr(1);
        if arith {
            res.set_bit(7, msb);
        }

        self.set_r8(reg, res);
        self.set_flag(Flags::Z, res == 0);
        self.set_flag(Flags::N, false);
        self.set_flag(Flags::H, false);
        self.set_flag(Flags::C, lsb);
    }

    pub fn swap_bits(&mut self, reg: Regs) {
        let val = self.get_r8(reg);
        let res = (val << 4) | (val >> 4);

        self.set_r8(reg, res);
        self.set_flag(Flags::Z, res == 0);
        self.set_flag(Flags::N, false);
        self.set_flag(Flags::H, false);
        self.set_flag(Flags::C, false);
    }

    pub fn test_bit(&mut self, reg: Regs, bit: u8) {
        let byte = self.get_r8(reg);
        let val = byte.get_bit(bit);

        self.set_flag(Flags::Z, !val);
        self.set_flag(Flags::N, false);
        self.set_flag(Flags::H, true);
    }

    pub fn write_bit(&mut self, reg: Regs, bit: u8, set: bool) {
        let mut byte = self.get_r8(reg);
        byte.set_bit(bit, set);
        self.set_r8(reg, byte);
    }

    pub fn set_halted(&mut self, halted: bool) {
        let if_reg = self.read_ram(IF) & 0x1F;
        let ie_reg = self.read_ram(IE) & 0x1F;

        if halted {
            if !self.irq_enabled {
                if (if_reg & ie_reg) != 0 {
                    self.halt_bug = true;
                    self.halted = false;
                } else {
                    self.halted = true;
                }
            } else {
                self.halted = true;
            }
        } else {
            self.halted = false;
        }
    }

    pub fn tick(&mut self) -> bool {
        self.last_read = None;
        self.last_write = None;
        self.pending_draw = false;

        if self.halted {
            self.internal_delay();
            let if_reg = self.read_ram(IF) & 0x1F;
            let ie_reg = self.read_ram(IE) & 0x1F;
            if (if_reg & ie_reg) != 0 {
                self.halted = false;
            }
        } else {
            let prev_ime_scheduled = self.ime_scheduled;
            opcodes::execute(self);
            if prev_ime_scheduled {
                self.irq_enabled = true;
                self.ime_scheduled = false;
            }
        }

        if let Some(irq) = self.check_irq() {
            self.trigger_irq(irq);
        }

        self.pending_draw
    }

    fn check_irq(&mut self) -> Option<Interrupts> {
        if !self.irq_enabled {
            return None;
        }

        let if_reg = self.read_ram(IF) & 0x1F;
        let ie_reg = self.read_ram(IE) & 0x1F;
        let irq_flags = if_reg & ie_reg;

        if irq_flags == 0 {
            return None;
        }

        for (i, irq) in IRQ_PRIORITIES.iter().enumerate() {
            if irq_flags.get_bit(i as u8) {
                return Some(*irq);
            }
        }
        None
    }

    fn enable_irq_type(&mut self, irq: Interrupts, enabled: bool) {
        let mut if_reg = self.read_ram(IF);
        match irq {
            Interrupts::Vblank => if_reg.set_bit(0, enabled),
            Interrupts::Stat => if_reg.set_bit(1, enabled),
            Interrupts::Timer => if_reg.set_bit(2, enabled),
            Interrupts::Serial => if_reg.set_bit(3, enabled),
            Interrupts::Joypad => if_reg.set_bit(4, enabled),
        }
        self.write_ram(IF, if_reg | 0xE0);
    }

    fn trigger_irq(&mut self, irq: Interrupts) {
        self.halted = false;
        self.irq_enabled = false;
        self.enable_irq_type(irq, false);

        self.internal_delay();
        self.internal_delay();

        let vector = irq.get_vector();
        self.push(self.pc);
        self.set_pc(vector);
    }

    pub fn load_rom(&mut self, rom: &[u8]) {
        self.bus.load_rom(rom);
    }

    pub fn render(&self) -> [u8; DISPLAY_BUFFER] {
        self.bus.render()
    }

    pub fn press_button(&mut self, button: Buttons, pressed: bool) {
        self.bus.press_button(button, pressed);
        self.enable_irq_type(Interrupts::Joypad, true);
    }

    pub fn get_audio_samples(&mut self) -> Vec<f32> {
        self.bus.apu.sample_buffer.drain(..).collect()
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Regs {
    A,
    B,
    C,
    D,
    E,
    F,
    H,
    L,
    HL,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Regs16 {
    AF,
    BC,
    DE,
    HL,
    SP,
}

pub enum Flags {
    Z,
    N,
    H,
    C,
}