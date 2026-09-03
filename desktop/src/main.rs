mod config;
mod debug;
mod gui;

use std::env;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::process::exit;
use std::time::Instant;

use crate::config::{AppConfig, ColorPalette};
use crate::debug::Debugger;
use crate::gui::EmulatorUiState;

use egui::{Color32, ColorImage, TextureHandle, TextureOptions};
use egui_sdl2_gl::{DpiScaling, ShaderVersion};
use sdl2::audio::{AudioQueue, AudioSpecDesired};
use sdl2::controller::GameController;
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::video::FullscreenType;

use gb_core::cpu::Cpu;
use gb_core::utils::{SCREEN_HEIGHT, SCREEN_WIDTH};

const MENU_HEIGHT: u32 = 24;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    let mut config = AppConfig::load();

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let audio_subsystem = sdl_context.audio().unwrap();
    let controller_subsystem = sdl_context.game_controller().unwrap();

    let desired_spec = AudioSpecDesired {
        freq: Some(44100),
        channels: Some(2),
        samples: Some(1024),
    };
    let audio_queue: AudioQueue<f32> = audio_subsystem.open_queue(None, &desired_spec).unwrap();
    audio_queue.resume();

    let mut active_controllers: Vec<GameController> = Vec::new();
    for i in 0..controller_subsystem.num_joysticks().unwrap_or(0) {
        if controller_subsystem.is_game_controller(i) {
            if let Ok(controller) = controller_subsystem.open(i) {
                active_controllers.push(controller);
            }
        }
    }

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
    gl_attr.set_context_version(3, 3);
    gl_attr.set_double_buffer(true);

    let initial_width = (SCREEN_WIDTH as u32) * config.scale_factor;
    let initial_height = ((SCREEN_HEIGHT as u32) * config.scale_factor) + MENU_HEIGHT;

    let mut window = video_subsystem
        .window("Game Boy Emulator", initial_width, initial_height)
        .opengl()
        .resizable()
        .position_centered()
        .build()?;

    let _gl_context = window.gl_create_context()?;
    gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const std::ffi::c_void);

    let swap_interval = if config.frame_limiter { 1 } else { 0 };
    video_subsystem.gl_set_swap_interval(swap_interval).ok();

    let (mut egui_painter, mut egui_state) =
        egui_sdl2_gl::with_sdl2(&window, ShaderVersion::Default, DpiScaling::Default);
    let egui_ctx = egui::Context::default();

    let blank_image = ColorImage::new(
        [SCREEN_WIDTH as usize, SCREEN_HEIGHT as usize],
        Color32::from_rgb(155, 188, 15),
    );
    let mut gb_screen_texture: TextureHandle = egui_ctx.load_texture(
        "gb_screen",
        blank_image,
        TextureOptions::NEAREST,
    );

    let mut events = sdl_context.event_pump().unwrap();
    let mut gbd = Debugger::new();
    let mut gb: Box<Cpu> = Box::new(Cpu::new());
    let mut ui_state = EmulatorUiState::from_config(&config);

    let mut current_rom_bytes: Option<Vec<u8>> = None;
    let mut current_title: String = String::new();
    let mut save_state_slots: Vec<Option<Box<Cpu>>> = vec![None; 9]; // 1-indexed slots 1..=8

    if args.len() > 1 {
        let filename = &args[1];
        let rom = load_rom(filename);
        gb.load_rom(&rom);
        current_title = gb.get_title();
        load_battery_save(&mut gb, &current_title);
        current_rom_bytes = Some(rom);
        config.add_recent_rom(filename);
    }

    let mut pixel_buffer = vec![Color32::BLACK; (SCREEN_WIDTH as usize) * (SCREEN_HEIGHT as usize)];
    let mut prev_frame_buffer = vec![Color32::BLACK; (SCREEN_WIDTH as usize) * (SCREEN_HEIGHT as usize)];
    let mut is_fullscreen = false;
    let start_time = Instant::now();

    'gameloop: loop {
        egui_state.input.time = Some(start_time.elapsed().as_secs_f64());

        for event in events.poll_iter() {
            match &event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    if !current_title.is_empty() {
                        write_battery_save(&mut gb, &current_title);
                    }
                    break 'gameloop;
                }
                Event::Window {
                    win_event: sdl2::event::WindowEvent::SizeChanged(w, h),
                    ..
                } => {
                    egui_painter.update_screen_rect((*w as u32, *h as u32));
                }
                Event::ControllerDeviceAdded { which, .. } => {
                    if let Ok(c) = controller_subsystem.open(*which) {
                        active_controllers.push(c);
                    }
                }
                Event::ControllerButtonDown { button, .. } => {
                    if let Some(btn) = AppConfig::controller_button_to_button(*button) {
                        gb.press_button(btn, true);
                    }
                }
                Event::ControllerButtonUp { button, .. } => {
                    if let Some(btn) = AppConfig::controller_button_to_button(*button) {
                        gb.press_button(btn, false);
                    }
                }
                // Hold Tab or Space for fast-forward
                Event::KeyDown {
                    keycode: Some(Keycode::Tab | Keycode::Space),
                    repeat: false,
                    ..
                } => {
                    if !egui_ctx.wants_keyboard_input() {
                        ui_state.fast_forward_held = true;
                    }
                }
                Event::KeyUp {
                    keycode: Some(Keycode::Tab | Keycode::Space),
                    ..
                } => {
                    ui_state.fast_forward_held = false;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F11),
                    ..
                } => {
                    ui_state.toggle_fullscreen = true;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Return),
                    keymod,
                    ..
                } if keymod.contains(Mod::LALTMOD) || keymod.contains(Mod::RALTMOD) => {
                    ui_state.toggle_fullscreen = true;
                }
                // F1-F8: Load state; Shift + F1-F8: Save state
                Event::KeyDown {
                    keycode: Some(key),
                    keymod,
                    ..
                } if matches!(
                    key,
                    Keycode::F1
                        | Keycode::F2
                        | Keycode::F3
                        | Keycode::F4
                        | Keycode::F5
                        | Keycode::F6
                        | Keycode::F7
                        | Keycode::F8
                ) => {
                    let slot = match key {
                        Keycode::F1 => 1,
                        Keycode::F2 => 2,
                        Keycode::F3 => 3,
                        Keycode::F4 => 4,
                        Keycode::F5 => 5,
                        Keycode::F6 => 6,
                        Keycode::F7 => 7,
                        Keycode::F8 => 8,
                        _ => unreachable!(),
                    };
                    ui_state.current_slot = slot;
                    if keymod.contains(Mod::LSHIFTMOD) || keymod.contains(Mod::RSHIFTMOD) {
                        ui_state.save_state_requested = Some(slot);
                    } else {
                        ui_state.load_state_requested = Some(slot);
                    }
                }
                Event::KeyDown {
                    keycode: Some(key), ..
                } => {
                    if let Some(ref target) = ui_state.listening_for_bind.take() {
                        let name = key.name();
                        match target.as_str() {
                            "Up" => config.keybinds.up = name,
                            "Down" => config.keybinds.down = name,
                            "Left" => config.keybinds.left = name,
                            "Right" => config.keybinds.right = name,
                            "A" => config.keybinds.a = name,
                            "B" => config.keybinds.b = name,
                            "Start" => config.keybinds.start = name,
                            "Select" => config.keybinds.select = name,
                            _ => {}
                        }
                        config.save();
                    } else if !egui_ctx.wants_keyboard_input() {
                        if let Some(btn) = config.key_to_button(*key) {
                            gb.press_button(btn, true);
                        }
                    }
                }
                Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    if let Some(btn) = config.key_to_button(*key) {
                        gb.press_button(btn, false);
                    }
                }
                _ => {}
            }
            egui_state.process_input(&window, event.clone(), &mut egui_painter);
        }

        // Fullscreen toggle
        if ui_state.toggle_fullscreen {
            ui_state.toggle_fullscreen = false;
            is_fullscreen = !is_fullscreen;
            let mode = if is_fullscreen {
                FullscreenType::Desktop
            } else {
                FullscreenType::Off
            };
            let _ = window.set_fullscreen(mode);
        }

        // Handle Save State (Slot Cache & saves/<Game>/slot{N}.ss{N})
        if let Some(slot) = ui_state.save_state_requested.take() {
            if current_rom_bytes.is_some() && (1..=8).contains(&slot) {
                if !current_title.is_empty() {
                    save_state_to_disk(&gb, &current_title, slot);
                }
                save_state_slots[slot] = Some(gb.clone());
            }
        }

        // Handle Load State (Disk fallback to Slot Cache)
        if let Some(slot) = ui_state.load_state_requested.take() {
            if (1..=8).contains(&slot) {
                if !current_title.is_empty() {
                    if let Some(loaded_gb) = load_state_from_disk(&current_title, slot) {
                        gb = loaded_gb;
                        save_state_slots[slot] = Some(gb.clone());
                    } else if let Some(ref snapshot) = save_state_slots[slot] {
                        gb = snapshot.clone();
                    }
                } else if let Some(ref snapshot) = save_state_slots[slot] {
                    gb = snapshot.clone();
                }
            }
        }

        // Apply scale change
        if let Some(scale) = ui_state.requested_scale.take() {
            if is_fullscreen {
                let _ = window.set_fullscreen(FullscreenType::Off);
                is_fullscreen = false;
            }
            let new_w = (SCREEN_WIDTH as u32) * scale;
            let new_h = ((SCREEN_HEIGHT as u32) * scale) + MENU_HEIGHT;
            let _ = window.set_size(new_w, new_h);
            egui_painter.update_screen_rect((new_w, new_h));
        }

        // Frame limiter (forced off if turbo/fast-forward is held)
        let target_swap = if ui_state.frame_limiter && !ui_state.fast_forward_held { 1 } else { 0 };
        video_subsystem.gl_set_swap_interval(target_swap).ok();

        // Handle ROM selection
        if let Some(path) = ui_state.pending_rom_path.take() {
            if let Ok(rom) = std::fs::read(&path) {
                if !current_title.is_empty() {
                    write_battery_save(&mut gb, &current_title);
                }
                gb = Box::new(Cpu::new());
                gb.load_rom(&rom);
                current_title = gb.get_title();
                load_battery_save(&mut gb, &current_title);
                current_rom_bytes = Some(rom);
                save_state_slots = vec![None; 9];
                config.add_recent_rom(&path.to_string_lossy());
            }
        }

        // Reset
        if ui_state.should_reset {
            ui_state.should_reset = false;
            if let Some(ref rom) = current_rom_bytes {
                if !current_title.is_empty() {
                    write_battery_save(&mut gb, &current_title);
                }
                gb = Box::new(Cpu::new());
                gb.load_rom(rom);
                load_battery_save(&mut gb, &current_title);
            }
        }

        // Emulation step with fast-forward frame skipping
        if !ui_state.is_paused && current_rom_bytes.is_some() {
            // Apply live APU channel mutes
            gb.set_channel_enabled(1, ui_state.ch1_enabled);
            gb.set_channel_enabled(2, ui_state.ch2_enabled);
            gb.set_channel_enabled(3, ui_state.ch3_enabled);
            gb.set_channel_enabled(4, ui_state.ch4_enabled);

            let is_turbo = ui_state.fast_forward_held;
            let frames_to_run = if is_turbo {
                4
            } else {
                match ui_state.speed_multiplier {
                    0 => 4,
                    s => s.max(1),
                }
            };

            for frame_idx in 0..frames_to_run {
                tick_until_draw(&mut gb, &mut gbd, &current_title);

                let mut samples = gb.get_audio_samples();
                if !is_turbo {
                    if ui_state.master_volume != 1.0 {
                        for sample in &mut samples {
                            *sample *= ui_state.master_volume;
                        }
                    }

                    if audio_queue.size() < 44100 * 2 {
                        let _ = audio_queue.queue_audio(&samples);
                    }
                } else {
                    // Prevent audio buffer backlog latency during turbo hold
                    audio_queue.clear();
                }

                // Skip intermediate frame LCD conversions; render only the final frame
                if frame_idx == frames_to_run - 1 {
                    let frame = gb.render();
                    let mut p_idx = 0;
                    for chunk in frame.chunks_exact(4) {
                        let lum = (chunk[0] as f32 * 0.299 + chunk[1] as f32 * 0.587 + chunk[2] as f32 * 0.114) as u8;
                        let mapped_color = apply_palette(lum, ui_state.palette);

                        let final_color = if ui_state.lcd_ghosting {
                            let prev = prev_frame_buffer[p_idx];
                            Color32::from_rgb(
                                ((mapped_color.r() as f32 * 0.65) + (prev.r() as f32 * 0.35)) as u8,
                                ((mapped_color.g() as f32 * 0.65) + (prev.g() as f32 * 0.35)) as u8,
                                ((mapped_color.b() as f32 * 0.65) + (prev.b() as f32 * 0.35)) as u8,
                            )
                        } else {
                            mapped_color
                        };

                        pixel_buffer[p_idx] = final_color;
                        prev_frame_buffer[p_idx] = final_color;
                        p_idx += 1;
                    }

                    gb_screen_texture.set(
                        ColorImage {
                            size: [SCREEN_WIDTH as usize, SCREEN_HEIGHT as usize],
                            pixels: pixel_buffer.clone(),
                        },
                        TextureOptions::NEAREST,
                    );
                }
            }
        }

        // Render egui
        let full_output = egui_ctx.run(egui_state.input.take(), |ctx| {
            ui_state.draw_menu_bar(ctx, &mut config);

            egui::CentralPanel::default().show(ctx, |ui| {
                let avail_size = ui.available_size();
                let image_size = if ui_state.keep_aspect_ratio {
                    let aspect = SCREEN_WIDTH as f32 / SCREEN_HEIGHT as f32;
                    if avail_size.x / avail_size.y > aspect {
                        egui::vec2(avail_size.y * aspect, avail_size.y)
                    } else {
                        egui::vec2(avail_size.x, avail_size.x / aspect)
                    }
                } else {
                    avail_size
                };

                ui.centered_and_justified(|ui| {
                    ui.image(&gb_screen_texture, image_size);
                });
            });
        });

        unsafe {
            gl::Viewport(0, 0, window.size().0 as i32, window.size().1 as i32);
            gl::ClearColor(0.08, 0.08, 0.08, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        let clipped_meshes = egui_ctx.tessellate(full_output.shapes);
        egui_painter.paint_jobs(
            None,
            full_output.textures_delta,
            clipped_meshes,
        );

        window.gl_swap_window();
    }

    Ok(())
}

fn get_save_state_path(title: &str, slot: usize) -> PathBuf {
    let sanitized_title: String = title
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            c => c,
        })
        .collect();

    let folder_name = if sanitized_title.trim().is_empty() {
        "unknown_game"
    } else {
        sanitized_title.trim()
    };

    let dir_path = std::path::Path::new("saves").join(folder_name);
    let _ = std::fs::create_dir_all(&dir_path);

    dir_path.join(format!("slot{}.ss{}", slot, slot))
}

fn save_state_to_disk(gb: &Cpu, title: &str, slot: usize) {
    let path = get_save_state_path(title, slot);
    if let Ok(file) = OpenOptions::new().write(true).create(true).truncate(true).open(&path) {
        let mut writer = BufWriter::new(file);
        let _ = bincode::serialize_into(&mut writer, gb);
    }
}

fn load_state_from_disk(title: &str, slot: usize) -> Option<Box<Cpu>> {
    let path = get_save_state_path(title, slot);
    if let Ok(file) = File::open(&path) {
        let mut reader = BufReader::new(file);
        if let Ok(state) = bincode::deserialize_from::<_, Box<Cpu>>(&mut reader) {
            return Some(state);
        }
    }
    None
}

fn apply_palette(lum: u8, palette: ColorPalette) -> Color32 {
    let shade = match lum {
        0..=63 => 0,
        64..=127 => 1,
        128..=191 => 2,
        _ => 3,
    };

    match palette {
        ColorPalette::PeaGreen => match shade {
            0 => Color32::from_rgb(15, 56, 15),
            1 => Color32::from_rgb(48, 98, 48),
            2 => Color32::from_rgb(139, 172, 15),
            _ => Color32::from_rgb(155, 188, 15),
        },
        ColorPalette::Pocket => match shade {
            0 => Color32::from_rgb(20, 20, 20),
            1 => Color32::from_rgb(86, 86, 86),
            2 => Color32::from_rgb(160, 160, 160),
            _ => Color32::from_rgb(230, 230, 230),
        },
        ColorPalette::Oled => match shade {
            0 => Color32::from_rgb(0, 0, 0),
            1 => Color32::from_rgb(60, 60, 60),
            2 => Color32::from_rgb(170, 170, 170),
            _ => Color32::from_rgb(255, 255, 255),
        },
    }
}

fn load_rom(path: &str) -> Vec<u8> {
    let mut buffer: Vec<u8> = Vec::new();
    let mut f = File::open(path).expect("Error opening ROM file");
    f.read_to_end(&mut buffer).expect("Error loading ROM");
    buffer
}

fn tick_until_draw(gb: &mut Cpu, gbd: &mut Debugger, gamename: &str) {
    loop {
        let render = gb.tick();

        gbd.check_exec_breakpoints(gb.get_pc());
        if let Some(addr) = gb.get_read() {
            gbd.check_read_breakpoints(addr);
        }
        if let Some(addr) = gb.get_write() {
            gbd.check_write_breakpoints(addr);
        }

        if gbd.is_debugging() {
            gbd.print_info();
            let quit = gbd.debugloop(gb);
            if quit {
                exit(0);
            }
        }

        if render {
            break;
        }
    }

    if !gamename.is_empty() && gb.is_battery_dirty() {
        write_battery_save(gb, gamename);
    }
}

fn write_battery_save(gb: &mut Cpu, gamename: &str) {
    if gb.has_battery() {
        let battery_data = gb.get_battery_data();
        let filename = format!("{}.sav", gamename);

        if let Ok(mut file) = OpenOptions::new().write(true).create(true).open(filename) {
            let _ = file.write_all(battery_data);
            gb.clean_battery();
        }
    }
}

fn load_battery_save(gb: &mut Cpu, gamename: &str) {
    if gb.has_battery() {
        let mut battery_data: Vec<u8> = Vec::new();
        let filename = format!("{}.sav", gamename);

        if let Ok(mut f) = OpenOptions::new().read(true).open(filename) {
            if f.read_to_end(&mut battery_data).is_ok() {
                gb.set_battery_data(&battery_data);
            }
        }
    }
}