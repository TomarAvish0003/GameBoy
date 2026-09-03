pub mod rtc;
use rtc::Rtc;
use crate::utils::BitOps;
use serde::{Serialize, Deserialize};

pub const ROM_START: u16 = 0x0000;
pub const ROM_STOP: u16 = 0x7FFF;
pub const EXT_RAM_START: u16 = 0xA000;
pub const EXT_RAM_STOP: u16 = 0xBFFF;

const ROM_BANK_LOW_START: u16 = 0x2000;
const ROM_BANK_LOW_STOP: u16 = 0x2FFF;
const ROM_BANK_HIGH_START: u16 = 0x3000;
const ROM_BANK_HIGH_STOP: u16 = 0x3FFF;

const ROM_BANK_SIZE: usize = 0x4000;
const RAM_BANK_SIZE: usize = 0x2000;
const MBC2_ROM_CONTROL_BIT: u8 = 8;

const RAM_ENABLE_START: u16 = 0x0000;
const RAM_ENABLE_STOP: u16 = 0x1FFF;
const ROM_BANK_NUM_START: u16 = 0x2000;
const ROM_BANK_NUM_STOP: u16 = 0x3FFF;
const RAM_BANK_NUM_START: u16 = 0x4000;
const RAM_BANK_NUM_STOP: u16 = 0x5FFF;
const ROM_RAM_MODE_START: u16 = 0x6000;
const ROM_RAM_MODE_STOP: u16 = 0x7FFF;

const RAM_SIZES: [usize; 6] = [
    0,
    2,
    8,
    32,
    128,
    64,
];

const RAM_SIZE_ADDR: usize = 0x0149;

#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum MBC {
    NONE,
    MBC1,
    MBC2,
    MBC3,
    MBC5,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Cart {
    rom: Vec<u8>,
    ram: Vec<u8>,
    rom_bank: u16,
    ram_bank: u8,
    mbc: MBC,
    rtc: Rtc,
    rom_mode: bool,
    ram_enabled: bool,
    has_battery: bool,
}

impl Cart {
    pub fn new() -> Self {
        Self {
            rom: Vec::new(),
            ram: Vec::new(),
            rom_bank: 1,
            ram_bank: 0,
            mbc: MBC::NONE,
            rtc: Rtc::new(),
            rom_mode: true,
            ram_enabled: false,
            has_battery: false,
        }
    }

    pub fn has_battery(&self) -> bool {
        self.has_battery
    }

    pub fn has_external_ram(&self) -> bool {
        !self.ram.is_empty()
    }

    pub fn get_mbc(&self) -> MBC {
        if self.rom.len() < 0x148 {
            return MBC::NONE;
        }
        match self.rom[0x0147] {
            0x00 => MBC::NONE,
            0x01..=0x03 => MBC::MBC1,
            0x05 | 0x06 => MBC::MBC2,
            0x0F..=0x13 => MBC::MBC3,
            0x19..=0x1E => MBC::MBC5,
            _ => MBC::NONE,
        }
    }

    fn init_ext_ram(&mut self) {
        if self.rom.len() <= RAM_SIZE_ADDR {
            return;
        }
        let mut ram_size_idx = self.rom[RAM_SIZE_ADDR] as usize;

        // some headers do not report their external RAM cap correctly
        if self.has_external_ram() && ram_size_idx == 0 {
            ram_size_idx = 1;
        }

        if self.mbc == MBC::MBC2 {
            self.ram = vec![0; 512]; // always 512 bytes of ram on chip
        } else if ram_size_idx < RAM_SIZES.len() {
            let ram_size = RAM_SIZES[ram_size_idx] * 1024;
            self.ram = vec![0; ram_size];
        }
    }

    pub fn load_cart(&mut self, rom: &[u8]) {
        self.rom = rom.to_vec();
        self.mbc = self.get_mbc();

        // Battery presence check based on cartridge header type
        if self.rom.len() > 0x0147 {
            self.has_battery = matches!(
                self.rom[0x0147],
                0x03 | 0x06 | 0x09 | 0x0D | 0x0F | 0x10 | 0x13 | 0x1B | 0x1E
            );
        }

        self.init_ext_ram();
    }

    pub fn read_cart(&self, addr: u16) -> u8 {
        if (addr as usize) < ROM_BANK_SIZE {
            self.rom.get(addr as usize).copied().unwrap_or(0xFF)
        } else {
            let rel_addr = (addr as usize) - ROM_BANK_SIZE;
            let bank_addr = (self.rom_bank as usize) * ROM_BANK_SIZE + rel_addr;
            self.rom.get(bank_addr).copied().unwrap_or(0xFF)
        }
    }

    pub fn write_cart(&mut self, addr: u16, val: u8) {
        match self.mbc {
            MBC::NONE => {}
            MBC::MBC1 => self.mbc1_write_rom(addr, val),
            MBC::MBC2 => self.mbc2_write_rom(addr, val),
            MBC::MBC3 => self.mbc3_write_rom(addr, val),
            MBC::MBC5 => self.mbc5_write_rom(addr, val),
        }
    }

    fn mbc5_write_rom(&mut self, addr: u16, val: u8) {
        match addr {
            RAM_ENABLE_START..=RAM_ENABLE_STOP => {
                self.ram_enabled = val == 0x0A;
            }
            ROM_BANK_LOW_START..=ROM_BANK_LOW_STOP => {
                self.rom_bank = (self.rom_bank & 0xFF00) | (val as u16);
            }
            ROM_BANK_HIGH_START..=ROM_BANK_HIGH_STOP => {
                self.rom_bank.set_bit(8, val.get_bit(0));
            }
            RAM_BANK_NUM_START..=RAM_BANK_NUM_STOP => {
                self.ram_bank = val & 0x0F;
            }
            _ => {}
        }
    }

    fn mbc3_write_rom(&mut self, addr: u16, val: u8) {
        match addr {
            RAM_ENABLE_START..=RAM_ENABLE_STOP => {
                self.ram_enabled = val == 0x0A;
            }
            ROM_BANK_NUM_START..=ROM_BANK_NUM_STOP => {
                let bank = if val == 0 { 1 } else { val as u16 & 0x7F };
                self.rom_bank = bank;
            }
            RAM_BANK_NUM_START..=RAM_BANK_NUM_STOP => {
                self.ram_bank = val;
            }
            ROM_RAM_MODE_START..=ROM_RAM_MODE_STOP => {
                self.rtc.write_byte(self.ram_bank, val);
            }
            _ => {}
        }
    }

    fn mbc2_write_rom(&mut self, addr: u16, val: u8) {
        let bank_swap = addr.get_bit(MBC2_ROM_CONTROL_BIT);
        if bank_swap {
            let bank = (val & 0x0F) as u16;
            self.rom_bank = if bank == 0 { 1 } else { bank };
        } else {
            self.ram_enabled = val == 0x0A;
        }
    }

    fn mbc1_write_rom(&mut self, addr: u16, val: u8) {
        match addr {
            RAM_ENABLE_START..=RAM_ENABLE_STOP => {
                self.ram_enabled = val == 0x0A;
            }
            ROM_BANK_NUM_START..=ROM_BANK_NUM_STOP => {
                let mut bank = (val & 0x1F) as u16;
                if bank == 0 {
                    bank = 1;
                }
                self.rom_bank = (self.rom_bank & 0xE0) | bank;
            }
            RAM_BANK_NUM_START..=RAM_BANK_NUM_STOP => {
                let bits = val & 0b11;
                if self.rom_mode {
                    self.rom_bank = (self.rom_bank & 0x1F) | ((bits as u16) << 5);
                } else {
                    self.ram_bank = bits;
                }
            }
            ROM_RAM_MODE_START..=ROM_RAM_MODE_STOP => {
                self.rom_mode = val == 0;
            }
            _ => {}
        }
    }

    pub fn read_ram(&self, addr: u16) -> u8 {
        match self.mbc {
            MBC::NONE | MBC::MBC1 | MBC::MBC2 | MBC::MBC5 => self.read_ram_helper(addr),
            MBC::MBC3 => self.mbc3_read_ram(addr),
        }
    }

    fn mbc3_read_ram(&self, addr: u16) -> u8 {
        if self.rtc.is_enabled() && (self.ram_bank >= 0x08 && self.ram_bank <= 0x0C) {
            self.rtc.read_byte(self.ram_bank)
        } else {
            self.read_ram_helper(addr)
        }
    }

    fn read_ram_helper(&self, addr: u16) -> u8 {
        if !self.ram_enabled || self.ram.is_empty() {
            return 0xFF;
        }
        let rel_addr = (addr - EXT_RAM_START) as usize;
        let bank_addr = (self.ram_bank as usize) * RAM_BANK_SIZE + rel_addr;
        self.ram.get(bank_addr).copied().unwrap_or(0xFF)
    }

    pub fn write_ram(&mut self, addr: u16, val: u8) {
        match self.mbc {
            MBC::NONE => {
                if !self.ram.is_empty() {
                    let rel_addr = (addr - EXT_RAM_START) as usize;
                    if rel_addr < self.ram.len() {
                        self.ram[rel_addr] = val;
                    }
                }
            }
            MBC::MBC1 | MBC::MBC5 => self.write_ram_helper(addr, val),
            MBC::MBC2 => {
                if self.ram_enabled && !self.ram.is_empty() {
                    let rel_addr = ((addr - EXT_RAM_START) % 512) as usize;
                    self.ram[rel_addr] = val & 0x0F;
                }
            }
            MBC::MBC3 => self.mbc3_write_ram(addr, val),
        }
    }

    fn mbc3_write_ram(&mut self, addr: u16, val: u8) {
        match self.ram_bank {
            0x00..=0x03 => self.write_ram_helper(addr, val),
            0x08..=0x0C => {
                if self.ram_enabled {
                    self.rtc.write_byte(self.ram_bank, val);
                }
            }
            _ => {}
        }
    }

    fn write_ram_helper(&mut self, addr: u16, val: u8) {
        if self.ram_enabled && !self.ram.is_empty() {
            let rel_addr = (addr - EXT_RAM_START) as usize;
            let ram_addr = (self.ram_bank as usize) * RAM_BANK_SIZE + rel_addr;
            if ram_addr < self.ram.len() {
                self.ram[ram_addr] = val;
            }
        }
    }

    pub fn get_battery_data(&self) -> &[u8] {
        &self.ram
    }

    pub fn set_battery_data(&mut self, data: &[u8]) {
        if self.ram.len() == data.len() {
            self.ram.copy_from_slice(data);
        }
    }

    pub fn get_title(&self) -> String {
        if self.rom.len() < 0x0143 {
            return "GAMEBOY".to_string();
        }
        let title_bytes = &self.rom[0x0134..0x0143];
        String::from_utf8_lossy(title_bytes)
            .trim_matches(char::from(0))
            .to_string()
    }
}