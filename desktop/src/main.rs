use std::env;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::process::exit;

mod debug;
use crate::debug::Debugger;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

use gb_core::cpu::Cpu;
use gb_core::io::Buttons;
use gb_core::utils::{DISPLAY_BUFFER, SCREEN_HEIGHT, SCREEN_WIDTH};

const SCALE: u32 = 3;

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() == 1 {
        println!("Please specify a ROM location: cargo run path/to/game");
        return;
    }

    let mut gbd = Debugger::new();
    let mut gb = Cpu::new();
    let filename = &args[1];
    let rom = load_rom(filename);
    gb.load_rom(&rom);

    let title = gb.get_title();
    load_battery_save(&mut gb, &title);

    // SDL2 Initialization
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window(
            &format!("Game Boy Emulator - {}", title),
            SCREEN_WIDTH as u32 * SCALE,
            SCREEN_HEIGHT as u32 * SCALE,
        )
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let mut events = sdl_context.event_pump().unwrap();

    'gameloop: loop {
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    write_battery_save(&mut gb, &title);
                    break 'gameloop;
                }
                Event::KeyDown {
                    keycode: Some(key), ..
                } => handle_key(&mut gb, key, true),
                Event::KeyUp {
                    keycode: Some(key), ..
                } => handle_key(&mut gb, key, false),
                _ => {}
            }
        }

        tick_until_draw(&mut gb, &mut gbd, &title);

        let frame = gb.render();
        draw_screen(&frame, &mut canvas);
    }
}

fn handle_key(gb: &mut Cpu, key: Keycode, pressed: bool) {
    match key {
        Keycode::Z => gb.press_button(Buttons::A, pressed),
        Keycode::X => gb.press_button(Buttons::B, pressed),
        Keycode::Return => gb.press_button(Buttons::Start, pressed),
        Keycode::Space => gb.press_button(Buttons::Select, pressed),
        Keycode::Right => gb.press_button(Buttons::Right, pressed),
        Keycode::Left => gb.press_button(Buttons::Left, pressed),
        Keycode::Up => gb.press_button(Buttons::Up, pressed),
        Keycode::Down => gb.press_button(Buttons::Down, pressed),
        _ => {}
    }
}

fn load_rom(path: &str) -> Vec<u8> {
    let mut buffer: Vec<u8> = Vec::new();
    let mut f = File::open(path).expect("Error opening ROM file");
    f.read_to_end(&mut buffer).expect("Error loading ROM");
    buffer
}

fn draw_screen(data: &[u8], canvas: &mut Canvas<Window>) {
    for i in (0..DISPLAY_BUFFER).step_by(4) {
        canvas.set_draw_color(Color::RGB(data[i], data[i + 1], data[i + 2]));
        let pixel = i / 4;
        let x = (pixel % (SCREEN_WIDTH as usize)) as u32;
        let y = (pixel / (SCREEN_WIDTH as usize)) as u32;

        let rect = Rect::new((x * SCALE) as i32, (y * SCALE) as i32, SCALE, SCALE);
        let _ = canvas.fill_rect(rect);
    }
    canvas.present();
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

    if gb.is_battery_dirty() {
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