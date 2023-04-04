extern crate sdl2;

mod hardware;
mod render;
mod font;

use crate::font::FONT_SET;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use std::collections::HashSet;

use std::{thread, time};

fn main() -> Result<(), String> {
    let mut renderer = render::Render::new("Chip8", hardware::CHIP8_WIDTH * hardware::MULTIPLIER, hardware::CHIP8_HEIGHT * hardware::MULTIPLIER, true)?;
    let mut chip8 = hardware::Chip8::new();

    chip8.load_ram(&FONT_SET, 0x50);
    //chip8.load_ram(include_bytes!("../IBM Logo.ch8"), 0x200);
    chip8.load_ram(include_bytes!("../chip8-test-suite.ch8"), 0x200);

    //chip8.load_ram(&[0x05], 0x1FF);

    chip8.start();

    renderer.sound.resume();

    let mut fixedstep = fixedstep::FixedStep::start(60.0);

    'running: loop {
        while fixedstep.update() {
            chip8.decrease_timers();
            if chip8.tim_snd == 0 {
                renderer.sound.pause();
            }
        }

        for event in renderer.event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown { keycode: Some(keycode), .. } => {
                    chip8.set_input(keycode, true);
                },
                Event::KeyUp { keycode: Some(keycode), .. } => {
                    chip8.set_input(keycode, false);
                },
                _ => {}
            }
        }

        chip8.cycle();

        if chip8.vram_changed {
            renderer.update(chip8.get_vram())?;
            chip8.vram_changed = false;
        }

        thread::sleep(time::Duration::from_millis(2));
    }
    Ok(())
}