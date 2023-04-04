mod hardware;
mod font;

use crate::font::FONT_SET;

use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Rust8".to_owned(),
        fullscreen: false,
        window_width: (hardware::CHIP8_WIDTH * hardware::MULTIPLIER) as i32,
        window_height: (hardware::CHIP8_HEIGHT * hardware::MULTIPLIER) as i32,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut chip8 = hardware::Chip8::new();

    chip8.load_ram(&FONT_SET, 0x50);
    //chip8.load_ram(include_bytes!("../IBM Logo.ch8"), 0x200);
    chip8.load_ram(include_bytes!("../chip8-test-suite.ch8"), 0x200);

    //chip8.load_ram(&[0x05], 0x1FF);

    chip8.start();

    let mut fixedstep = fixedstep::FixedStep::start(60.0);

    clear_background(BLACK);

    loop {
        while fixedstep.update() {
            chip8.decrease_timers();
        }

        chip8.cycle();

        for (y, row) in chip8.get_vram().rows_iter().enumerate() {
            for (x, px) in row.enumerate() {
                if *px {
                    draw_rectangle((x * hardware::MULTIPLIER as usize) as f32, (y * hardware::MULTIPLIER as usize) as f32, hardware::MULTIPLIER as f32, hardware::MULTIPLIER as f32, GREEN);
                }
                else {
                    draw_rectangle((x * hardware::MULTIPLIER as usize) as f32, (y * hardware::MULTIPLIER as usize) as f32, hardware::MULTIPLIER as f32, hardware::MULTIPLIER as f32, BLACK);
                }
            }
        }

        next_frame().await
    }
}