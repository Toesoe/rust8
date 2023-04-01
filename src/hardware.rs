use rand::Rng;
use std::io;
use std::thread::sleep;
use std::time::{Duration, Instant};

use array2d::{Array2D, Error};
use sdl2::keyboard::Keycode;

pub const CHIP8_WIDTH: u32 = 64;
pub const CHIP8_HEIGHT: u32 = 32;
pub const MULTIPLIER: u32 = 20;

pub const RAM_SIZE: usize = 4096;

pub const V_REG_COUNT: usize = 16;
pub const STACK_SIZE: usize = 16;

pub enum PC {
    // keep current PC value
    Keep = 0,
    // increment PC with 2
    Step = 1,
    // increment PC with 4
    Skip = 2,
}

pub struct CPU {
    pub pc: usize,
    pub sp: usize,
    pub stack: Vec<usize>,
    pub i: usize,
    pub v: [u8; V_REG_COUNT],
    pub tim_delay: u8,
    pub tim_snd: u8,
    pub ram: [u8; RAM_SIZE],
    pub disp: Array2D<bool>,
    pub disp_changed: bool,
    pub key: u8,
    pub key_pressed: bool,
    pub key_processed: bool,
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            pc: 0x0,
            sp: 0x0,
            stack: Vec::with_capacity(STACK_SIZE),
            i: 0x0,
            v: [0x0; V_REG_COUNT],
            tim_delay: 255,
            tim_snd: 255,
            ram: [0x0; RAM_SIZE],
            disp: Array2D::filled_with(false, CHIP8_HEIGHT as usize, CHIP8_WIDTH as usize),
            disp_changed: false,
            key: 0xFF,
            key_pressed: false,
            key_processed: false,
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

    pub fn set_input(&mut self, key: Keycode) {
        self.key = match key {
            Keycode::Num1 => Some(0x1),
            Keycode::Num2 => Some(0x2),
            Keycode::Num3 => Some(0x3),
            Keycode::Num4 => Some(0xc),
            Keycode::Q => Some(0x4),
            Keycode::W => Some(0x5),
            Keycode::E => Some(0x6),
            Keycode::R => Some(0xd),
            Keycode::A => Some(0x7),
            Keycode::S => Some(0x8),
            Keycode::D => Some(0x9),
            Keycode::F => Some(0xe),
            Keycode::Z => Some(0xa),
            Keycode::X => Some(0x0),
            Keycode::C => Some(0xb),
            Keycode::V => Some(0xf),
            _ => None,
        }
        .unwrap();
        self.key_pressed = true;
    }

    pub fn cycle(&mut self) -> Result<(), Error> {
        let opcode = (self.ram[self.pc] as usize) << 8 | (self.ram[self.pc + 1] as usize);
        let mut nibs: Vec<usize> = Vec::new();

        let mut step_pc: PC = PC::Keep;

        for n in 0..4 {
            let nib = (opcode & (0xF000 >> (n * 4))) >> (12 - (n * 4));
            nibs.push(nib);
        }

        println!("executing {:#0x}", opcode);

        step_pc = match nibs[0] {
            0x0 => self.op_0xxx(opcode),
            0x1 => {
                self.pc = (opcode & 0xFFF) as usize;
                PC::Keep
            } // Jump to address NNN
            0x2 => self.op_2xxx(opcode),
            0x3 | 0x4 | 0x5 => self.op_3xxx_4xxx_5xxx(&nibs),
            0x6 | 0x7 => self.op_6xxx_7xxx(&nibs),
            0x8 => self.op_8xxx(&nibs),
            0x9 => {
                if self.v[nibs[1]] != self.v[nibs[2]] {
                    PC::Skip
                } else {
                    PC::Step
                }
            } // skip if Vx != Vy
            0xA => {
                self.i = (opcode & 0xFFF) as usize;
                PC::Step
            } // Store memory address NNN in register I
            0xB => {
                self.pc = (opcode & 0xFFF) + self.v[0] as usize;
                PC::Keep
            } // Jump to address NNN + V0
            0xC => {
                self.v[nibs[1]] =
                    rand::thread_rng().gen_range(0..=255) & (((nibs[2] << 4) | nibs[3]) as u8);
                PC::Step
            } // Set VX to a random number with a mask of NN
            0xD => self.op_Dxxx(&nibs),
            0xE => self.op_Exxx(&nibs),
            0xF => self.op_Fxxx(&nibs),
            _ => todo!("{:#0x} instr", nibs[0]),
        };

        match step_pc {
            PC::Step => self.pc += 2,
            PC::Skip => self.pc += 4,
            PC::Keep => {}
            _ => panic!("Unhandled PC option"),
        }

        Ok(())
    }

    /**
     * `0NNN`: Execute machine language subroutine at address NNN
     * `00E0`: Clear the screen
     * `00EE`: Return from a subroutine
     */
    fn op_0xxx(&mut self, opcode: usize) -> PC {
        let mut ret = PC::Step;

        match opcode {
            0xE0 => {
                self.disp =
                    Array2D::filled_with(false, CHIP8_HEIGHT as usize, CHIP8_WIDTH as usize);
                self.disp_changed = true;
            }
            0xEE => {
                self.pc = self.stack.pop().unwrap();
            }
            _ => {
                self.pc = opcode;
                ret = PC::Keep;
            }
        };

        return ret;
    }

    /**
     * `2NNN`: Execute subroutine starting at address NNN
     */
    fn op_2xxx(&mut self, opcode: usize) -> PC {
        self.stack.push(self.pc);
        self.pc = (opcode & 0xFFF) as usize;

        return PC::Keep;
    }

    /**
     * `3XNN`: Skip the following instruction if the value of register VX equals NN
     * `4XNN`: Skip the following instruction if the value of register VX is not equal to NN
     * `5XY0`: Skip the following instruction if the value of register VX is equal to the value of register VY
     */
    fn op_3xxx_4xxx_5xxx(&mut self, nibs: &Vec<usize>) -> PC {
        let mut ret = PC::Step;

        if nibs[0] == 0x3 && (self.v[nibs[1]] == ((nibs[2] << 4) | nibs[3]) as u8) {
            ret = PC::Skip;
        } else if nibs[0] == 0x4 && (self.v[nibs[1]] != ((nibs[2] << 4) | nibs[3]) as u8) {
            ret = PC::Skip;
        } else if nibs[0] == 0x5 && (self.v[nibs[1]] == self.v[nibs[2]]) {
            ret = PC::Skip;
        }
        return ret;
    }

    /**
     * 6XNN: Store number NN in register VX
     * 7XNN: Add the value NN to register VX
     */
    fn op_6xxx_7xxx(&mut self, nibs: &Vec<usize>) -> PC {
        if nibs[0] == 0x6 {
            self.v[nibs[1]] = ((nibs[2] << 4) | nibs[3]) as u8;
        } else if nibs[0] == 0x7 {
            let val = ((nibs[2] << 4) | nibs[3]) as u8;
            if self.v[nibs[1]].checked_add(val) == None {
                self.v[nibs[1]] = ((self.v[nibs[1]] as u16 + val as u16) - 256) as u8;
            }
        }

        return PC::Step;
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
    fn op_8xxx(&mut self, nibs: &Vec<usize>) -> PC {
        match nibs[3] {
            0 => self.v[nibs[1]] = self.v[nibs[2]],
            1 => self.v[nibs[1]] |= self.v[nibs[2]],
            2 => self.v[nibs[1]] &= self.v[nibs[2]],
            3 => self.v[nibs[1]] ^= self.v[nibs[2]],

            4 => {
                if self.v[nibs[1]].checked_add(self.v[nibs[2]]) == None {
                    self.v[15] = 0x01;
                    self.v[nibs[1]] =
                        ((self.v[nibs[1]] as u16 + self.v[nibs[2]] as u16) - 256) as u8;
                }
            }

            5 => {
                if self.v[nibs[1]].checked_sub(self.v[nibs[2]]) == None {
                    self.v[15] = 0x01;
                    self.v[nibs[1]] = (self.v[nibs[1]] as i16 - self.v[nibs[2]] as i16) as u8;
                }
            }

            6 => {
                self.v[15] = self.v[nibs[2]] & 1;
                self.v[nibs[1]] = self.v[nibs[2]] >> 1;
            }

            7 => {
                let value = self.v[nibs[2]].checked_sub(self.v[nibs[1]]);
                if value == None {
                    self.v[15] = 0x01;
                    self.v[nibs[1]] = 0;
                } else {
                    self.v[nibs[1]] = value.unwrap();
                }
            }
            0xE => {
                self.v[15] = (self.v[nibs[2]] >> 7) & 1;
                self.v[nibs[1]] = self.v[nibs[2]] << 1;
            }
            _ => panic!("invalid instruction {:#0x} for 0x8xxx", nibs[3]),
        }

        return PC::Step;
    }

    /**
    * `DXYN`: Draw a sprite at position VX, VY with N bytes of sprite data starting at the address stored in I
    - The corresponding graphic on the screen will be eight pixels wide (bits in 1 byte) and N pixels high
    - Set VF to 01 if any set pixels are changed to unset, and 00 otherwise
    */
    fn op_Dxxx(&mut self, nibs: &Vec<usize>) -> PC {
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
                    self.disp.set(y + row_count, x + n, px_val).unwrap();
                }
            }

            row_count += 1;
            sprite_height -= 1;

            if y + row_count == CHIP8_HEIGHT as usize {
                break;
            }
        }

        return PC::Step;
    }

    /**
     * `EX9E` Skip the following instruction if the key corresponding to the hex value currently stored in register VX is pressed
     * `EXA1` Skip the following instruction if the key corresponding to the hex value currently stored in register VX is not pressed
     */
    fn op_Exxx(&mut self, nibs: &Vec<usize>) -> PC {
        let mut ret = PC::Step;

        if !self.key_processed {
            match ((nibs[2] << 4) | nibs[3]) as u8 {
                0x9E => {
                    if self.key == self.v[nibs[1]] {
                        ret = PC::Skip;
                    }
                }
                0xA1 => {
                    if self.key != self.v[nibs[1]] {
                        ret = PC::Skip;
                    }
                }
                _ => panic!("invalid instruction"),
            }
            
            self.key_processed = true;
        }

        return ret;
    }

    /**
     * `FXxx` Misc register operations.
     */
    fn op_Fxxx(&mut self, nibs: &Vec<usize>) -> PC {
        let mut ret = PC::Step;

        match ((nibs[2] << 4) | nibs[3]) as u8 {
            // Store the current value of the delay timer in register VX
            0x07 => self.v[nibs[1]] = self.tim_delay,

            // Wait for a keypress and store the result in register VX
            0x0A => {
                if self.key_pressed {
                    self.v[nibs[1]] = self.key;
                    self.key_pressed = false;
                    self.key_processed = false;
                } else {
                    ret = PC::Keep;
                }
            }

            // Set the delay timer to the value of register VX
            0x15 => self.tim_delay = self.v[nibs[1]],

            // Set the sound timer to the value of register VX
            0x18 => self.tim_snd = self.v[nibs[1]],

            // Add the value stored in register VX to register I
            0x1E => {
                if self.i.checked_add(self.v[nibs[1]] as usize) == None {
                    self.i = 65535;
                }
            }

            // Set I to the memory address of the sprite data corresponding to the hexadecimal digit stored in register VX
            0x29 => self.i = (self.v[nibs[1]] * 0x5) as usize,

            // Store the BCD equivalent of the value stored in register VX at addresses I, I + 1, and I + 2
            0x33 => {
                let mut val = self.v[nibs[1]] as u32;
                let mut count = 0;
                let mut res: u32 = 0;

                while val > 0 {
                    res |= (val % 10) << (count << 2);
                    val /= 10;
                    count += 1;
                }

                self.ram[self.i + 2] = (res & 0xF) as u8;
                self.ram[self.i + 1] = ((res & 0xF0) >> 4) as u8;
                self.ram[self.i] = ((res & 0xF00) >> 8) as u8;
            }

            // Store the values of registers V0 to VX inclusive in memory starting at address I.
            // I is set to I + X + 1 after operation
            0x55 => {
                for n in 0..=nibs[1] as usize {
                    self.ram[self.i + n] = self.v[n];
                }
                self.i += (self.v[nibs[1]] + 1) as usize;
            }

            // Fill registers V0 to VX inclusive with the values stored in memory starting at address I
            // I is set to I + X + 1 after operation
            0x65 => {
                for n in 0..=nibs[1] as usize {
                    self.v[n] = self.ram[self.i + n];
                }
                self.i += (self.v[nibs[1]] + 1) as usize;
            }
            _ => panic!(
                "invalid instruction {:#0x} for 0xFxxx",
                ((nibs[2] << 4) | nibs[3]) as u8
            ),
        }

        return ret;
    }
}
