extern crate sdl2;

use crate::hardware;
use crate::hardware::CHIP8_WIDTH;

use sdl2::render::Canvas;
use sdl2::rect::Rect;
use sdl2::rect::Point;
use sdl2::pixels::Color;

pub struct Drawing {
    pub context: sdl2::Sdl,
    pub canvas: Canvas<sdl2::video::Window>,
    pub draw_grid: bool,
}

impl Drawing {
    pub fn new(draw_grid: bool) -> Result<Drawing, String> {
        let context = sdl2::init().unwrap();

        let video_subsystem = context.video().unwrap();
    
        let window = video_subsystem
            .window("rust-sdl2 demo: Video", hardware::CHIP8_WIDTH * hardware::MULTIPLIER, hardware::CHIP8_HEIGHT * hardware::MULTIPLIER)
            .position_centered()
            .opengl()
            .build()
            .unwrap();
    
        let canvas = window.into_canvas().build().unwrap();

        Ok(Drawing {context, canvas, draw_grid})
    }
    
    /**
     * Update canvas with VRAM data.
     */
    pub fn update(&mut self, cpu: &mut hardware::CPU) -> Result<(), String> {
        let vram_ref = cpu.get_vram();

        for (y, row) in vram_ref.rows_iter().enumerate() {
            for (x, px) in row.enumerate() {
                if *px {
                    self.canvas.set_draw_color(Color::GREEN);
                }
                else {
                    self.canvas.set_draw_color(Color::BLACK);
                }
                self.draw_dot(x as i32, y as i32)?;
            }
        }
        if self.draw_grid
        {
            self.draw_grid()?;
        }
        self.canvas.present();
        Ok(())
    }

    /**
     * Will draw a grid for debugging. Every 8x4 block will be marked with red lines.
     */
    pub fn draw_grid(&mut self) -> Result<(), String> {
        let mut n = hardware::MULTIPLIER as usize;

        while n < (hardware::CHIP8_WIDTH * hardware::MULTIPLIER) as usize {
            let start = Point::new(n as i32, 0);
            let end = Point::new(n as i32, (hardware::CHIP8_HEIGHT * hardware::MULTIPLIER) as i32);
            if n % 160 == 0 {
                self.canvas.set_draw_color(Color::RED);
            }
            else {
                self.canvas.set_draw_color(Color::GRAY);
            }
            self.canvas.draw_line(start, end)?;
            n += hardware::MULTIPLIER as usize;
        }

        n = hardware::MULTIPLIER as usize;

        while n < (hardware::CHIP8_HEIGHT * hardware::MULTIPLIER) as usize {
            let start = Point::new(0 as i32, n as i32);
            let end = Point::new((hardware::CHIP8_WIDTH * hardware::MULTIPLIER) as i32, n as i32);
            if n % 80 == 0 {
                self.canvas.set_draw_color(Color::RED);
            }
            else {
                self.canvas.set_draw_color(Color::GRAY);
            }
            self.canvas.draw_line(start, end)?;
            n += hardware::MULTIPLIER as usize;
        }
        Ok(())
    }

    /**
     * Will draw a single pixel at X/Y.
     */
    fn draw_dot(&mut self, x_in: i32, y_in: i32) -> Result<(), String> {
        let point = Point::new(x_in, y_in);
        self.canvas.fill_rect(Rect::new(
            point.x * hardware::MULTIPLIER as i32,
            point.y * hardware::MULTIPLIER as i32,
            hardware::MULTIPLIER,
            hardware::MULTIPLIER,
        ))?;

        Ok(())
    }
}