use gb_core::cpu::Cpu;
use gb_core::io::Buttons;
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct GB {
    cpu: Cpu,
}

#[wasm_bindgen]
impl GB {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        GB { cpu: Cpu::new() }
    }

    #[wasm_bindgen]
    pub fn load_rom(&mut self, data: Uint8Array) {
        let mut rom = vec![0u8; data.byte_length() as usize];
        data.copy_to(&mut rom);
        self.cpu.load_rom(&rom);
    }

    #[wasm_bindgen]
    pub fn step_frame(&mut self) {
        while !self.cpu.tick() {}
    }

    #[wasm_bindgen]
    pub fn get_screen(&self) -> Vec<u8> {
        self.cpu.render().to_vec()
    }

    #[wasm_bindgen]
    pub fn get_audio_samples(&mut self) -> Vec<f32> {
        self.cpu.get_audio_samples()
    }

    #[wasm_bindgen]
    pub fn press_button(&mut self, key: &str, pressed: bool) {
        if let Some(button) = key2btn(key) {
            self.cpu.press_button(button, pressed);
        }
    }
}

fn key2btn(key: &str) -> Option<Buttons> {
    match key {
        "ArrowDown" => Some(Buttons::Down),
        "ArrowUp" => Some(Buttons::Up),
        "ArrowRight" => Some(Buttons::Right),
        "ArrowLeft" => Some(Buttons::Left),
        "Enter" => Some(Buttons::Start),
        "Backspace" | "Shift" => Some(Buttons::Select),
        "x" | "X" => Some(Buttons::A),
        "z" | "Z" => Some(Buttons::B),
        _ => None,
    }
}