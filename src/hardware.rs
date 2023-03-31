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
        let mut nibs: Vec<usize> = Vec::new();

        for n in 0..4{
            let nib = (opcode & (0xF000 >> (n * 4))) >> (12 - (n * 4));
            nibs.push(nib);
        }

        match nibs[0] {
            0x0 => self.op_0xxx(opcode),
            0x1 => self.pc = opcode & 0xFFF, // Jump to address NNN
            0x2 => self.op_2xxx(opcode),
            0x3 | 0x4 | 0x5 => self.op_3xxx_4xxx_5xxx(&nibs),
            0x6 | 0x7 => self.op_6xxx_7xxx(&nibs),
            0x8 => self.op_8xxx(&nibs),
            0xA => self.i = opcode & 0xFFF, // Store memory address NNN in register I
            0xD => self.op_Dxxx(&nibs)?,
            _ => todo!("{:#0x} instr", nibs[0]),
        }
        Ok(())
    }

    /**
     * `0NNN`: Execute machine language subroutine at address NNN
     * `00E0`: Clear the screen
     * `00EE`: Return from a subroutine
     */
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

    /**
     * `2NNN`: Execute subroutine starting at address NNN
     */
    fn op_2xxx(&mut self, opcode: usize) {
        self.stack.push(self.pc);
        self.pc = (opcode & 0xFFF) as usize;
    }

    /**
     * `3XNN`: Skip the following instruction if the value of register VX equals NN
     * `4XNN`: Skip the following instruction if the value of register VX is not equal to NN
     * `5XY0`: Skip the following instruction if the value of register VX is equal to the value of register VY
     */
    fn op_3xxx_4xxx_5xxx(&mut self, nibs: &Vec<usize>) {
        if nibs[0] == 0x3 && self.v[nibs[1]] == ((nibs[2] << 4) | nibs[3]) as u8 {
            self.pc += 2;
        } else if nibs[0] == 0x4 && self.v[nibs[1]] != ((nibs[2] << 4) | nibs[3]) as u8 {
            self.pc += 2;
        } else if nibs[0] == 0x5 && self.v[nibs[1]] == self.v[nibs[2]] {
            self.pc += 2;
        }
    }

    /**
     * 6XNN: Store number NN in register VX
     * 7XNN: Add the value NN to register VX
     */
    fn op_6xxx_7xxx(&mut self, nibs: &Vec<usize>) {
        if nibs[0] == 0x6 {
            self.v[nibs[1]] = ((nibs[2] << 4) | nibs[3]) as u8;
        } else if nibs[0] == 0x7 {
            if self.v[nibs[1]].checked_add(((nibs[2] << 4) | nibs[3]) as u8) == None {
                self.v[nibs[1]] = 255;
            }
        }
    }

    /**
     * _Arithmetic/bitwise operations._
     * `8XY0` Store the value of register VY in register VX
     * `8XY1` Set VX to VX OR VY
     * `8XY2` Set VX to VX AND VY
     * `8XY3` Set VX to VX XOR VY
     * `8XY4` Add the value of register VY to register VX
       - Set VF to 01 if a carry occurs
       - Set VF to 00 if a carry does not occur
     * `8XY5` Subtract the value of register VY from register VX
       - Set VF to 00 if a borrow occurs
       - Set VF to 01 if a borrow does not occur
     * `8XY6` Store the value of register VY shifted right one bit in register VX¹
       - Set register VF to the least significant bit prior to the shift
       - VY is unchanged
     * `8XY7` Set register VX to the value of VY minus VX
       - Set VF to 00 if a borrow occurs
       - Set VF to 01 if a borrow does not occur
     * `8XYE` Store the value of register VY shifted left one bit in register VX¹
       - Set register VF to the most significant bit prior to the shift
       - VY is unchanged
     */
    fn op_8xxx(&mut self, nibs: &Vec<usize>) {
        match nibs[3] {
            0 => self.v[nibs[1]] = self.v[nibs[2]],
            1 => self.v[nibs[1]] |= self.v[nibs[2]],
            2 => self.v[nibs[1]] &= self.v[nibs[2]],
            3 => self.v[nibs[1]] ^= self.v[nibs[2]],

            4 => {
                if self.v[nibs[1]].checked_add(self.v[nibs[2]]) == None {
                    self.v[15] = 0x01;
                    self.v[nibs[1]] = 255;
                }
            },

            5 => {
                if self.v[nibs[1]].checked_sub(self.v[nibs[2]]) == None {
                    self.v[15] = 0x01;
                    self.v[nibs[1]] = 0;
                }
            },

            6 => {
                self.v[15] = self.v[nibs[2]] & 1;
                self.v[nibs[1]] = self.v[nibs[2]] >> 1;
            },

            7 => {
                let value = self.v[nibs[2]].checked_sub(self.v[nibs[1]]);
                if value == None {
                    self.v[15] = 0x01;
                    self.v[nibs[1]] = 0;
                }
                else {
                    self.v[nibs[1]] = value.unwrap();
                }
            },
            0xE => {
                self.v[15] = (self.v[nibs[2]] >> 7) & 1;
                self.v[nibs[1]] = self.v[nibs[2]] << 1;
            },
            _ => todo!("unhandled"),
        }
    }

    /**
     * `DXYN`: Draw a sprite at position VX, VY with N bytes of sprite data starting at the address stored in I
     - The corresponding graphic on the screen will be eight pixels wide (bits in 1 byte) and N pixels high
     - Set VF to 01 if any set pixels are changed to unset, and 00 otherwise
     */
    fn op_Dxxx(&mut self, nibs: &Vec<usize>) -> Result<(), Error> {
        let x = usize::from(self.v[nibs[1]]);
        let y = usize::from(self.v[nibs[2]]);

        let mut sprite_height = nibs[3];
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
