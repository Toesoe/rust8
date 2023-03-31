use std::io;
use std::thread::sleep;
use std::time::{Duration, Instant};

use array2d::{Array2D, Error};

pub const CHIP8_WIDTH: u32 = 64;
pub const CHIP8_HEIGHT: u32 = 32;
pub const MULTIPLIER: u32 = 20;

pub const RAM_SIZE: usize = 4096;
pub const VRAM_SIZE: usize = (CHIP8_WIDTH as usize * CHIP8_HEIGHT as usize);

pub struct CPU {
    pub pc: usize,
    pub sp: usize,
    pub stack: Vec<usize>,
    pub i: usize,
    pub v: [u8; 16],
    pub tim_delay: u8,
    pub tim_snd: u8,
    pub ram: [u8; RAM_SIZE],
    pub disp: Array2D<bool>,
    pub disp_changed: bool,
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            pc: 0x0,
            sp: 0x0,
            stack: Vec::with_capacity(16),
            i: 0x0,
            v: [0x0; 16],
            tim_delay: 255,
            tim_snd: 255,
            ram: [0x0; RAM_SIZE],
            disp: Array2D::filled_with(false, CHIP8_HEIGHT as usize, CHIP8_WIDTH as usize),
            disp_changed: false,
        }
    }

    pub fn load_ram(&mut self, data: &[u8], addr: usize) {
        let len = data.len();
        self.ram[addr..addr + len].copy_from_slice(data);
    }

    pub fn start(&mut self) {
        self.pc = 0x200;
    }

    pub fn get_vram(&mut self) -> &Array2D<bool> {
        return &self.disp;
    }

    pub fn fetch(&mut self) -> usize {
        let instr = (self.ram[self.pc] as usize) << 8 | (self.ram[self.pc + 1] as usize);
        self.pc += 2;
        return instr;
    }

    pub fn decode(&mut self, opcode: usize) -> Result<(), Error> {
        let nibbles = (
            ((opcode & 0xF000) >> 12) as u8,
            ((opcode & 0x0F00) >> 8) as u8,
            ((opcode & 0x00F0) >> 4) as u8,
            (opcode & 0x000F) as u8,
        );

        match nibbles.0 {
            0x0 => self.op_0xxx(opcode),
            0x1 => self.pc = (opcode & 0xFFF) as usize,
            0x2 => self.op_2xxx(opcode),
            0x3 | 0x4 => self.op_3xxx_4xxx(nibbles),
            0x6 | 0x7 => self.op_6xxx_7xxx(nibbles),
            0xA => self.op_Axxx(opcode),
            0xD => self.op_Dxxx(nibbles)?,
            _ => todo!("instr"),
        }
        Ok(())
    }

    fn op_0xxx(&mut self, opcode: usize) {
        match opcode {
            0xE0 => {
                self.disp = Array2D::filled_with(false, CHIP8_HEIGHT as usize, CHIP8_WIDTH as usize);
                self.disp_changed = true;
            }
            0xEE => self.pc = self.stack.pop().unwrap(),
            _ => self.pc = opcode,
        };
    }

    fn op_2xxx(&mut self, opcode: usize) {
        self.stack.push(self.pc);
        self.pc = (opcode & 0xFFF) as usize;
    }

    fn op_3xxx_4xxx(&mut self, nibbles: (u8, u8, u8, u8)) {
        if nibbles.0 == 0x3 && self.v[nibbles.1 as usize] == ((nibbles.2 << 4) | nibbles.3) {
            self.pc += 2;
        } else if nibbles.0 == 0x4 && self.v[nibbles.1 as usize] != ((nibbles.2 << 4) | nibbles.3) {
            self.pc += 2;
        }
    }

    fn op_6xxx_7xxx(&mut self, nibbles: (u8, u8, u8, u8)) {
        if nibbles.0 == 0x6 {
            self.v[nibbles.1 as usize] = (nibbles.2 << 4) | nibbles.3;
        } else if nibbles.0 == 0x7 {
            // todo check overflow
            self.v[nibbles.1 as usize] += (nibbles.2 << 4) | nibbles.3;
        }
    }

    fn op_Axxx(&mut self, opcode: usize) {
        self.i = opcode & 0xFFF;
    }

    /**
     * DXYN -> draw sprite.
     * The corresponding graphic on the screen will be eight pixels wide (bits in 1 byte) and N pixels high.
     */
    fn op_Dxxx(&mut self, nibbles: (u8, u8, u8, u8)) -> Result<(), Error> {
        let x = usize::from(self.v[usize::from(nibbles.1)]);
        let y = usize::from(self.v[usize::from(nibbles.2)]);

        let mut sprite_height = usize::from(nibbles.3);
        let mut row_count = 0;

        self.v[15] = 0x0; // VF == 0
        self.disp_changed = true;

        // do some unpacking. each byte corresponds to 8 pixels
        while sprite_height > 0 {
            for n in 0..8 {
                if x + n == CHIP8_WIDTH as usize {
                    break;
                } else {
                    // take endianness into account :)
                    let px_val = (self.ram[self.i + row_count] & (1 << 7 - n)) != 0;

                    if *self.disp.get(y + row_count, x + n).unwrap() && px_val {
                        self.v[15] = 0x01; // VF == 1 when a pixel has been turned off
                    }
                    self.disp.set(y + row_count, x + n, px_val)?;
                }
            }

            row_count += 1;
            sprite_height -= 1;

            if y + row_count == CHIP8_HEIGHT as usize {
                break;
            }
        }
        Ok(())
    }
}
