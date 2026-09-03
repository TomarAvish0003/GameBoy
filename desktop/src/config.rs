use gb_core::io::Buttons;
use sdl2::controller::Button as ControllerButton;
use sdl2::keyboard::Keycode;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

const CONFIG_PATH: &str = "config.json";

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ColorPalette {
    PeaGreen,
    Pocket,
    Oled,
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self::PeaGreen
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KeybindConfig {
    pub up: String,
    pub down: String,
    pub left: String,
    pub right: String,
    pub a: String,
    pub b: String,
    pub start: String,
    pub select: String,
}

impl Default for KeybindConfig {
    fn default() -> Self {
        Self {
            up: "Up".to_string(),
            down: "Down".to_string(),
            left: "Left".to_string(),
            right: "Right".to_string(),
            a: "Z".to_string(),
            b: "X".to_string(),
            start: "Return".to_string(),
            select: "Space".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    pub scale_factor: u32,
    pub keep_aspect_ratio: bool,
    pub master_volume: f32,
    pub frame_limiter: bool,
    pub lcd_ghosting: bool,
    pub palette: ColorPalette,
    pub recent_roms: Vec<String>,
    pub keybinds: KeybindConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            scale_factor: 3,
            keep_aspect_ratio: true,
            master_volume: 1.0,
            frame_limiter: true,
            lcd_ghosting: false,
            palette: ColorPalette::default(),
            recent_roms: Vec::new(),
            keybinds: KeybindConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        if Path::new(CONFIG_PATH).exists() {
            if let Ok(mut file) = File::open(CONFIG_PATH) {
                let mut content = String::new();
                if file.read_to_string(&mut content).is_ok() {
                    if let Ok(cfg) = serde_json::from_str(&content) {
                        return cfg;
                    }
                }
            }
        }
        let default_cfg = AppConfig::default();
        default_cfg.save();
        default_cfg
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            if let Ok(mut file) = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(CONFIG_PATH)
            {
                let _ = file.write_all(json.as_bytes());
            }
        }
    }

    pub fn add_recent_rom(&mut self, path: &str) {
        self.recent_roms.retain(|p| p != path);
        self.recent_roms.insert(0, path.to_string());
        if self.recent_roms.len() > 5 {
            self.recent_roms.truncate(5);
        }
        self.save();
    }

    pub fn key_to_button(&self, key: Keycode) -> Option<Buttons> {
        let name = key.name();
        if name == self.keybinds.up {
            Some(Buttons::Up)
        } else if name == self.keybinds.down {
            Some(Buttons::Down)
        } else if name == self.keybinds.left {
            Some(Buttons::Left)
        } else if name == self.keybinds.right {
            Some(Buttons::Right)
        } else if name == self.keybinds.a {
            Some(Buttons::A)
        } else if name == self.keybinds.b {
            Some(Buttons::B)
        } else if name == self.keybinds.start {
            Some(Buttons::Start)
        } else if name == self.keybinds.select {
            Some(Buttons::Select)
        } else {
            None
        }
    }

    pub fn controller_button_to_button(btn: ControllerButton) -> Option<Buttons> {
        match btn {
            ControllerButton::DPadUp => Some(Buttons::Up),
            ControllerButton::DPadDown => Some(Buttons::Down),
            ControllerButton::DPadLeft => Some(Buttons::Left),
            ControllerButton::DPadRight => Some(Buttons::Right),
            ControllerButton::A => Some(Buttons::A),
            ControllerButton::B | ControllerButton::X => Some(Buttons::B),
            ControllerButton::Start => Some(Buttons::Start),
            ControllerButton::Back => Some(Buttons::Select),
            _ => None,
        }
    }
}