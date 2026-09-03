use crate::config::{AppConfig, ColorPalette};
use rfd::FileDialog;
use std::path::PathBuf;

pub struct EmulatorUiState {
    pub is_paused: bool,
    pub speed_multiplier: u32,
    pub scale_factor: u32,
    pub keep_aspect_ratio: bool,
    pub master_volume: f32,
    pub frame_limiter: bool,
    pub lcd_ghosting: bool,
    pub palette: ColorPalette,
    pub pending_rom_path: Option<PathBuf>,
    pub should_reset: bool,
    pub requested_scale: Option<u32>,
    pub toggle_fullscreen: bool,
    pub show_controls_window: bool,
    pub listening_for_bind: Option<String>,
    pub current_slot: usize,
    pub save_state_requested: Option<usize>,
    pub load_state_requested: Option<usize>,
    pub fast_forward_held: bool,
    pub ch1_enabled: bool,
    pub ch2_enabled: bool,
    pub ch3_enabled: bool,
    pub ch4_enabled: bool,
}

impl EmulatorUiState {
    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            is_paused: false,
            speed_multiplier: 1,
            scale_factor: config.scale_factor,
            keep_aspect_ratio: config.keep_aspect_ratio,
            master_volume: config.master_volume,
            frame_limiter: config.frame_limiter,
            lcd_ghosting: config.lcd_ghosting,
            palette: config.palette,
            pending_rom_path: None,
            should_reset: false,
            requested_scale: None,
            toggle_fullscreen: false,
            show_controls_window: false,
            listening_for_bind: None,
            current_slot: 1,
            save_state_requested: None,
            load_state_requested: None,
            fast_forward_held: false,
            ch1_enabled: true,
            ch2_enabled: true,
            ch3_enabled: true,
            ch4_enabled: true,
        }
    }

    pub fn draw_menu_bar(&mut self, ctx: &egui::Context, config: &mut AppConfig) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open ROM... (Ctrl+O)").clicked() {
                        if let Some(file) = FileDialog::new()
                            .add_filter("Game Boy ROM", &["gb", "dmg", "bin"])
                            .pick_file()
                        {
                            self.pending_rom_path = Some(file);
                        }
                        ui.close_menu();
                    }

                    ui.menu_button("Open Recent", |ui| {
                        if config.recent_roms.is_empty() {
                            ui.label("No recent files");
                        } else {
                            let mut clicked_path = None;
                            for path_str in &config.recent_roms {
                                let filename = std::path::Path::new(path_str)
                                    .file_name()
                                    .map(|s| s.to_string_lossy().to_string())
                                    .unwrap_or_else(|| path_str.clone());

                                if ui.button(filename).clicked() {
                                    clicked_path = Some(PathBuf::from(path_str));
                                }
                            }
                            if let Some(path) = clicked_path {
                                self.pending_rom_path = Some(path);
                                ui.close_menu();
                            }
                        }
                    });

                    ui.separator();
                    ui.menu_button("Save State Slot", |ui| {
                        for slot in 1..=8 {
                            let selected = self.current_slot == slot;
                            if ui.selectable_label(selected, format!("Slot {}", slot)).clicked() {
                                self.current_slot = slot;
                            }
                        }
                    });

                    if ui.button(format!("Quick Save (F5) [Slot {}]", self.current_slot)).clicked() {
                        self.save_state_requested = Some(self.current_slot);
                        ui.close_menu();
                    }

                    if ui.button(format!("Quick Load (F8) [Slot {}]", self.current_slot)).clicked() {
                        self.load_state_requested = Some(self.current_slot);
                        ui.close_menu();
                    }

                    ui.separator();
                    if ui.button("Reset (Ctrl+R)").clicked() {
                        self.should_reset = true;
                        ui.close_menu();
                    }

                    ui.separator();
                    if ui.button("Quit").clicked() {
                        std::process::exit(0);
                    }
                });

                ui.menu_button("Emulation", |ui| {
                    let pause_label = if self.is_paused { "Resume" } else { "Pause" };
                    if ui.button(pause_label).clicked() {
                        self.is_paused = !self.is_paused;
                        ui.close_menu();
                    }

                    if ui.checkbox(&mut self.frame_limiter, "Frame Limiter (VSync)").changed() {
                        config.frame_limiter = self.frame_limiter;
                        config.save();
                    }

                    ui.separator();
                    ui.label("Speed:");
                    ui.radio_value(&mut self.speed_multiplier, 1, "1x (Normal)");
                    ui.radio_value(&mut self.speed_multiplier, 2, "2x");
                    ui.radio_value(&mut self.speed_multiplier, 4, "4x");
                    ui.radio_value(&mut self.speed_multiplier, 8, "8x");
                    ui.radio_value(&mut self.speed_multiplier, 0, "Uncapped");
                });

                ui.menu_button("Video", |ui| {
                    if ui.button("Toggle Fullscreen (F11 / Alt+Enter)").clicked() {
                        self.toggle_fullscreen = true;
                        ui.close_menu();
                    }

                    if ui.checkbox(&mut self.keep_aspect_ratio, "Keep Aspect Ratio (10:9)").changed() {
                        config.keep_aspect_ratio = self.keep_aspect_ratio;
                        config.save();
                    }

                    if ui.checkbox(&mut self.lcd_ghosting, "LCD Ghosting Blend").changed() {
                        config.lcd_ghosting = self.lcd_ghosting;
                        config.save();
                    }

                    ui.separator();
                    ui.label("Color Palette:");
                    if ui.radio_value(&mut self.palette, ColorPalette::PeaGreen, "DMG Pea Green").clicked()
                        || ui.radio_value(&mut self.palette, ColorPalette::Pocket, "Pocket Grayscale").clicked()
                        || ui.radio_value(&mut self.palette, ColorPalette::Oled, "High-Contrast OLED").clicked()
                    {
                        config.palette = self.palette;
                        config.save();
                    }

                    ui.separator();
                    ui.label("Window Scale:");
                    for scale in 1..=5 {
                        let selected = self.scale_factor == scale;
                        if ui.selectable_label(selected, format!("{}x", scale)).clicked() {
                            self.scale_factor = scale;
                            self.requested_scale = Some(scale);
                            config.scale_factor = scale;
                            config.save();
                        }
                    }
                });

                ui.menu_button("Audio", |ui| {
                    if ui.add(egui::Slider::new(&mut self.master_volume, 0.0..=1.0).text("Master Volume")).changed() {
                        config.master_volume = self.master_volume;
                        config.save();
                    }

                    ui.separator();
                    ui.label("Channels:");
                    ui.checkbox(&mut self.ch1_enabled, "Channel 1 (Pulse 1)");
                    ui.checkbox(&mut self.ch2_enabled, "Channel 2 (Pulse 2)");
                    ui.checkbox(&mut self.ch3_enabled, "Channel 3 (Wave)");
                    ui.checkbox(&mut self.ch4_enabled, "Channel 4 (Noise)");
                });

                ui.menu_button("Controls", |ui| {
                    if ui.button("Keyboard Binds...").clicked() {
                        self.show_controls_window = true;
                        ui.close_menu();
                    }
                });
            });
        });

        // Controls Modal Dialog
        if self.show_controls_window {
            egui::Window::new("Controller & Keyboard Settings")
                .collapsible(false)
                .resizable(false)
                .open(&mut self.show_controls_window)
                .show(ctx, |ui| {
                    ui.heading("Keyboard Mappings");
                    ui.label("Click a button then press a key to rebind:");
                    ui.add_space(4.0);

                    let mut binds = [
                        ("Up", &mut config.keybinds.up),
                        ("Down", &mut config.keybinds.down),
                        ("Left", &mut config.keybinds.left),
                        ("Right", &mut config.keybinds.right),
                        ("A", &mut config.keybinds.a),
                        ("B", &mut config.keybinds.b),
                        ("Start", &mut config.keybinds.start),
                        ("Select", &mut config.keybinds.select),
                    ];

                    egui::Grid::new("binds_grid").num_columns(2).spacing([40.0, 6.0]).show(ui, |ui| {
                        for (label, key_val) in binds.iter_mut() {
                            ui.label(*label);
                            let text = if self.listening_for_bind.as_deref() == Some(*label) {
                                "[Press Key...]".to_string()
                            } else {
                                key_val.to_string()
                            };

                            if ui.button(text).clicked() {
                                self.listening_for_bind = Some(label.to_string());
                            }
                            ui.end_row();
                        }
                    });

                    ui.separator();
                    ui.label("Gamepads: Standard XInput/DirectInput D-Pad, A, B, Start, and Select are active automatically.");
                });
        }
    }
}