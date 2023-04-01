extern crate sdl2;

mod hardware;
mod draw;
mod font;

use crate::font::FONT_SET;

use draw::Drawing;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

fn main() -> Result<(), String> {
    let mut draw = Drawing::new(true)?;
    let mut cpu = hardware::CPU::new();

    cpu.load_ram(&FONT_SET, 0x50);
    cpu.load_ram(include_bytes!("../chip8-test-suite.ch8"), 0x200);

    cpu.start();

    let mut event_pump = draw.context.event_pump()?;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown { keycode: Some(keycode), .. } => {
                    match keycode {
                        Keycode::Num1 | Keycode::Num2 | Keycode::Num3 | Keycode::Num4 |
                        Keycode::Q    | Keycode::W    | Keycode::E    | Keycode::R    |
                        Keycode::A    | Keycode::S    | Keycode::D    | Keycode::F    |
                        Keycode::Z    | Keycode::X    | Keycode::C    | Keycode::V
                        => cpu.set_input(keycode),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        cpu.cycle();

        if cpu.disp_changed {
            draw.update(&mut cpu)?;
            cpu.disp_changed = false;
        }
    }
    Ok(())
}