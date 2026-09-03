use gb_core::cpu::Cpu;
use gb_core::io::Buttons;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct GB {
    cpu: Box<Cpu>,
    screen_buffer: Vec<u8>,
}

#[wasm_bindgen]
impl GB {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        GB {
            cpu: Box::new(Cpu::new()),
            screen_buffer: vec![0xFF; 160 * 144 * 4],
        }
    }

    #[wasm_bindgen]
    pub fn load_rom(&mut self, data: &[u8]) {
        self.cpu = Box::new(Cpu::new());
        self.cpu.load_rom(data);
    }

    #[wasm_bindgen]
    pub fn get_title(&self) -> String {
        self.cpu.get_title()
    }

    #[wasm_bindgen]
    pub fn step_frame(&mut self) {
        while !self.cpu.tick() {}
        self.screen_buffer.copy_from_slice(&self.cpu.render());
    }

    #[wasm_bindgen]
    pub fn get_screen_ptr(&self) -> *const u8 {
        self.screen_buffer.as_ptr()
    }

    #[wasm_bindgen]
    pub fn get_audio_samples(&mut self) -> Vec<f32> {
        self.cpu.get_audio_samples()
    }

    #[wasm_bindgen]
    pub fn set_channel_enabled(&mut self, channel: usize, enabled: bool) {
        self.cpu.set_channel_enabled(channel, enabled);
    }

    #[wasm_bindgen]
    pub fn press_button(&mut self, btn_idx: u8, pressed: bool) {
        let btn = match btn_idx {
            0 => Buttons::Right,
            1 => Buttons::Left,
            2 => Buttons::Up,
            3 => Buttons::Down,
            4 => Buttons::A,
            5 => Buttons::B,
            6 => Buttons::Select,
            7 => Buttons::Start,
            _ => return,
        };
        self.cpu.press_button(btn, pressed);
    }

    // Battery / SRAM Persistence
    #[wasm_bindgen]
    pub fn has_battery(&self) -> bool {
        self.cpu.has_battery()
    }

    #[wasm_bindgen]
    pub fn is_battery_dirty(&self) -> bool {
        self.cpu.is_battery_dirty()
    }

    #[wasm_bindgen]
    pub fn get_battery_data(&mut self) -> Vec<u8> {
        let data = self.cpu.get_battery_data().to_vec();
        self.cpu.clean_battery();
        data
    }

    #[wasm_bindgen]
    pub fn set_battery_data(&mut self, data: &[u8]) {
        self.cpu.set_battery_data(data);
    }

    // Save States (IndexedDB BLOBs)
    #[wasm_bindgen]
    pub fn save_state(&self) -> Result<Vec<u8>, JsValue> {
        bincode::serialize(&self.cpu).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen]
    pub fn load_state(&mut self, state_data: &[u8]) -> Result<(), JsValue> {
        let loaded: Box<Cpu> = bincode::deserialize(state_data)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.cpu = loaded;
        Ok(())
    }
}