extern crate sdl2;

use crate::hardware;

use sdl2::render::Canvas;
use sdl2::rect::Rect;
use sdl2::rect::Point;
use sdl2::pixels::Color;

use sdl2::audio::{AudioCallback, AudioSpecDesired, AudioDevice};

use array2d::Array2D;

pub struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // Generate a square wave
        for x in out.iter_mut() {
            *x = if self.phase <= 0.5 {
                self.volume
            } else {
                -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

pub struct Render {
    pub canvas: Canvas<sdl2::video::Window>,
    pub event_pump: sdl2::EventPump,
    pub timer: sdl2::TimerSubsystem,
    pub sound: AudioDevice<SquareWave>,
    pub width: u32,
    pub height: u32,
    pub draw_grid: bool,
}

impl Render {
    pub fn new(title: &str,
            width: u32,
            height: u32,
            draw_grid: bool
    ) -> Result<Render, String> {

     let context = sdl2::init()?;
     let video = context.video()?;
     let window = video.window(title, width, height)
         .position_centered()
         .opengl()
         .build()
         .map_err(|e| e.to_string())?;

     let canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
     let event_pump = context.event_pump()?;
     let timer_subsystem = context.timer()?;
     let audio_subsystem = context.audio()?;

     let desired_spec = AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1),  // mono
        samples: None       // default sample size
    };

    let audio_device = audio_subsystem.open_playback(None, &desired_spec, |spec| {
        // initialize the audio callback
        SquareWave {
            phase_inc: 440.0 / spec.freq as f32,
            phase: 0.0,
            volume: 0.25
        }
    }).unwrap();
     
     Ok(Render {
         canvas: canvas,
         event_pump: event_pump,
         timer: timer_subsystem,
         sound: audio_device,
         width: width,
         height: height,
         draw_grid: draw_grid,
     })
    }
    
    /**
     * Update canvas with VRAM data.
     */
    pub fn update(&mut self, chip8_vram: &Array2D<bool>) -> Result<(), String> {
        for (y, row) in chip8_vram.rows_iter().enumerate() {
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