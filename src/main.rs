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
    cpu.load_ram(include_bytes!("../IBM Logo.ch8"), 0x200);

    cpu.start();

    let mut event_pump = draw.context.event_pump()?;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }
        let instr = cpu.fetch();
        cpu.decode(instr);

        if cpu.disp_changed {
            draw.update(&mut cpu)?;
            cpu.disp_changed = false;
        }

        if cpu.pc == 0x284 {
            println!("ASD");
        }
    }
    Ok(())
}