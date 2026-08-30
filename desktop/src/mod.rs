use gb_core::utils::{SCREEN_WIDTH, SCREEN_HEIGHT};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::env;
use std::fs::File;
use std::io::Read;
use gb_core::io::Buttons;

const SCALE: u32 = 3;
const WINDOW_WIDTH: u32 = (SCREEN_WIDTH as u32) * SCALE;
const WINDOW_HEIGHT: u32 = (SCREEN_HEIGHT as u32) * SCALE;

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() == 1 {
        println!("Please specify a ROM location: cargo run path/to/game");
        return:
    }

    let filename = &args[1];
    let rom = load_rom(filename);

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsytem.window("GameBoy Emulator", WINDOW_WIDTH, WINDOW_HEIGHT).position_centered().opengl().build().unwrap();
    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    canvas.clear();
    canvas.present();

    let mut events = sdl_context.event_pump().unwrap();
    'gameloop: loop {
        for event in events.poll_iter() {
            match event {
                Event:: Quit{..} |
                Event::KeyDown{keycode: Some(Keycode::Escape), ..} => {
                    break 'gameloop;
                },
                Event::KeyDown{keycode: Some(Keycode::Space), ..} => {
                    gbd.set_debugging(true);
                },
                Event::KeyDown{keycode: Some(keycode), ..} => {
                    if let Some(button) = key2btn(keycode) {
                        gb.press_button(button, true);
                    }
                },
                Event::KeyUp{keycode: Some(keycode), ..} => {
                    if let Some(button) = key2btn(keycode) {
                        gb.press_button(button, false);
                    }
                },
                _ => {}
            }
        }

        sleep(Duration::from_millis(100));
    }
}

fn key2btn(key: Keycode) -> Option<Buttons> {
    match key {
        Keycode::Down => {Some(Buttons::Down)},
        Keycode::Up => {Some(Buttons::Up)},
        Keycode::Left => {Some(Buttons::Left)},
        Keycode::Right => {Some(Buttons::Right)},
        Keycode::Return => {Some(Buttons::Start)},
        Keycode::Backspace => {Some(Buttons::Select)},
        Keycode::X => {Some(Buttons::A)},
        Keycode::z => {Some(Buttons::B)},
        _ => {None}
    }
}

fn load_rom(path: &str) -> Vec<u8> {
    let mut buffer: Vec<u8> = Vec::new();

    let mut f = File::open(path).expect("Error opening ROM file");
    f.read_to_end(&mut buffer).expect("Error loading ROM");
    buffer
}