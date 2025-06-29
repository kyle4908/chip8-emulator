use std::{fs, mem};

use crate::nibbles::Nibbles;
pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;
// CHIP8 screen size is 64*32 pixels


const RAM_SIZE: usize = 4096;
// CHIP8 memory size is 4 kilobytes

const STACK_SIZE: usize = 16;
// kinda arbitrary, it doesn't necessarily need to be limited, I just did it for safety since most other emulators do it
// and the emulator will never realistically use all this anyway

const NUM_VARIABLE_REGISTERS: usize = 16;
// 16 variable registers in CHIP8

const FONT_SET_SIZE: usize = 80;

const FONT_SET: [u8; FONT_SET_SIZE] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

const NUM_KEYS: usize = 16;
// CHIP8 usually used on computers with hexidecimal keypads

pub struct Emulator {
    ram: [u8; RAM_SIZE],
    pub screen: [[bool; SCREEN_WIDTH]; SCREEN_HEIGHT], // bool because pixels can be either black or white
    pc: u16, // program counter, points to current instruction in memory, memory addresses are 16 bits
    i: u16, // index register, used to point to locations in memory
    stack: Vec<u16>, // stack for addresses
    // stack_pointer: u16, // points to the current index we are at on the "stack"
    delay_timer: u8,
    sound_timer: u8,
    variable_registers: [u8; NUM_VARIABLE_REGISTERS],
    keypad: [bool; NUM_KEYS],
    redraw_required: bool, // flag indicating a change to the screen was made
}

const START_ADDR: u16 = 0x200;
// CHIP8 programs are supposed to be loaded into memory after address 200

impl Emulator {
    pub fn new() -> Self {
        let mut emulator: Self = Self {
            ram: [0; RAM_SIZE],
            screen: [[false; SCREEN_WIDTH]; SCREEN_HEIGHT],
            pc: START_ADDR,
            i: 0,
            stack: Vec::new(),
            // stack_pointer: 0,
            delay_timer: 0,
            sound_timer: 0,
            variable_registers: [0; NUM_VARIABLE_REGISTERS],
            keypad: [false; NUM_KEYS],
            redraw_required: false
        };

        emulator.ram[..FONT_SET_SIZE].copy_from_slice(&FONT_SET);
        emulator
    }

    /// Load program into memory from the specified file
    pub fn load_file(&mut self, file: &str) {
        let program_bytes = fs::read(file).unwrap();
        for (i, byte) in (&program_bytes).into_iter().enumerate(){
            self.ram[START_ADDR as usize + i] = *byte;
        }
    }

    pub fn needs_redraw(&mut self) -> bool {
        if self.redraw_required{
            self.redraw_required = false;
            return true
        }
        false
    }

    /// Execute the instruction and do what it tells you
    pub fn execute(&mut self) {
        let decoded_operation: Nibbles = self.decode();

        match decoded_operation.category {
            0x0 => {
                match decoded_operation.nnn {
                    0x0E0 => self.clear_screen(),
                    0x0EE => self.subroutine_exit(),
                    _ => {}
                }
            }
            0x1 => self.jump(decoded_operation.nnn),
            0x2 => self.subroutine_call(decoded_operation.nnn),
            0x6 => self.set_register(decoded_operation.x, decoded_operation.nn),
            0x7 => self.add_to_register(decoded_operation.x, decoded_operation.nn),
            0xA => self.set_index_register(decoded_operation.nnn),
            0xD => self.display(decoded_operation.x, decoded_operation.y, decoded_operation.n),
            _ => {}
        }
    }

    /// Decode the instruction to find out what the emulator should do
    fn decode(&mut self) -> Nibbles {
        let instruction: u16 = self.fetch();
        Nibbles::new(instruction)
    }

    /// Fetch the instruction from memory at the current PC (program counter)
    fn fetch(&mut self) -> u16 {
        let mut instruction: u16 = (self.fetch_next_byte() as u16) << 8;
        instruction |= self.fetch_next_byte() as u16;
        instruction
    }

    /// Fetches the next byte from memory and increments the program counter
    fn fetch_next_byte(&mut self) -> u8 {
        let byte: u8 = self.ram[self.pc as usize];
        self.pc += 1;
        self.pc = self.pc % (RAM_SIZE as u16);
        byte
    }

    /// Clear the display, turning all pixels off to 0
    fn clear_screen(&mut self) {
        for i in 0..SCREEN_HEIGHT {
            for j in 0..SCREEN_WIDTH {
                self.screen[i][j] = false;
            }
        }
        self.redraw_required = true;
    }

    /// Set the PC counter to `memory_location` which is 12-bit, despite using u16 to represent it
    fn jump(&mut self, memory_location: u16) {
        self.pc = memory_location
    }

    /// Set the PC counter to `memory_location` which is 12-bit, despite using u16 to represent it
    /// and push the current PC to the stack, so the subroutine can return later
    fn subroutine_call(&mut self, memory_location: u16) {
        self.pc = memory_location;
        self.stack.push(memory_location);
    }

    /// Pop the last instruction from the stack and set the PC to it.
    /// used to return from a subroutine
    fn subroutine_exit(&mut self) {
        self.pc = self.stack.pop().expect("Expected stack to contain an instruction when exiting a subroutine");
    }

    /// Set register `reg` to `value`
    fn set_register(&mut self, reg: u8, value: u8) {
        self.variable_registers[reg as usize] = value;
    }

    /// Add `value` to register `reg`
    /// Do not set carry flag on overflow
    fn add_to_register(&mut self, reg: u8, value: u8) {
        let i: usize = reg as usize;
        self.variable_registers[i] = self.variable_registers[i].wrapping_add(value)
    }

    /// Set index register `i` to 12-bit value (represented using u16)
    fn set_index_register(&mut self, value: u16) {
        self.i = value
    }

    /// It will draw an N pixels tall sprite from the memory location that the I index register 
    /// is holding to the screen, at the horizontal X coordinate in VX and the Y coordinate in VY. 
    /// All the pixels that are “on” in the sprite will flip the pixels on the screen that it is 
    /// drawn to (from left to right, from most to least significant bit). If any pixels on the 
    /// screen were turned “off” by this, the VF flag register is set to 1. Otherwise, it’s set to 0.
    fn display(&mut self, x_reg: u8, y_reg: u8, sprite_height: u8) {
       let x_coord: usize = self.variable_registers[x_reg as usize] as usize % SCREEN_WIDTH;
       let y_coord: usize = self.variable_registers[y_reg as usize] as usize % SCREEN_HEIGHT;
       self.variable_registers[15] = 0;

       for i in 0..sprite_height {
        let sprite_row: u8 = self.ram[self.i as usize + i as usize];
        for j in 0..8 {
            let x = x_coord + j;
            let y = y_coord + i as usize;

            // starting at leftmost part of row and going to rightmost
            let sprite_bit_on = ((sprite_row >> (7 - j)) & 1) == 1;

            if sprite_bit_on && self.screen[y][x] {
                self.screen[y][x] = false;
                self.variable_registers[15] = 1;
            } else if sprite_bit_on && !self.screen[y][x] {
                self.screen[y][x] = true;
            }
        }
       }
       self.redraw_required = true;
    }
}
