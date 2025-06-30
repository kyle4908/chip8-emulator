use std::fs;

use crate::opcode::Opcode;
pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;
// CHIP8 screen size is 64*32 pixels

const RAM_SIZE: usize = 4096;
// CHIP8 memory size is 4 kilobytes

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
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

const NUM_KEYS: usize = 16;
// CHIP8 usually used on computers with hexidecimal keypads

pub struct Emulator {
    ram: [u8; RAM_SIZE],
    pub screen: [[bool; SCREEN_WIDTH]; SCREEN_HEIGHT], // bool because pixels can be either black or white
    pc: u16, // program counter, points to current instruction in memory, memory addresses are 16 bits
    i: u16,  // index register, used to point to locations in memory
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
            redraw_required: false,
        };

        emulator.ram[..FONT_SET_SIZE].copy_from_slice(&FONT_SET);
        emulator
    }

    /// Load program into memory from the specified file
    pub fn load_file(&mut self, file: &str) {
        let program_bytes = fs::read(file).unwrap();
        for (i, byte) in (program_bytes).iter().enumerate() {
            self.ram[START_ADDR as usize + i] = *byte;
        }
    }

    /// Tells if the emulator needs a redraw, automatically updated the redraw required
    /// flag back to false when a redraw is required
    pub fn needs_redraw(&mut self) -> bool {
        if self.redraw_required {
            self.redraw_required = false;
            return true;
        }
        false
    }

    /// Execute the instruction and do what it tells you
    pub fn execute(&mut self) {
        let decoded_operation: Opcode = self.decode();

        match decoded_operation.category {
            0x0 => match decoded_operation.nnn {
                0x0E0 => self.clear_screen(),
                0x0EE => self.subroutine_exit(),
                _ => {}
            },
            0x1 => self.jump(decoded_operation.nnn),
            0x2 => self.subroutine_call(decoded_operation.nnn),
            0x3 => self.skip_if_equal(decoded_operation.x, decoded_operation.nn),
            0x4 => self.skip_if_not_equal(decoded_operation.x, decoded_operation.nn),
            0x5 => match decoded_operation.n {
                0x0 => self.skip_if_regs_equal(decoded_operation.x, decoded_operation.y),
                _ => {}
            },
            0x6 => self.set_register_to_val(decoded_operation.x, decoded_operation.nn),
            0x7 => self.add_val_to_register(decoded_operation.x, decoded_operation.nn),
            0x8 => match decoded_operation.n {
                0x0 => self.set_register_to_register(decoded_operation.x, decoded_operation.y),
                0x1 => self.bitwise_or(decoded_operation.x, decoded_operation.y),
                0x2 => self.bitwise_and(decoded_operation.x, decoded_operation.y),
                0x3 => self.bitwise_xor(decoded_operation.x, decoded_operation.y),
                0x4 => self.add_register_to_register(decoded_operation.x, decoded_operation.y),
                0x5 => self.subtract_yregister_from_xregister(decoded_operation.x, decoded_operation.y),
                0x6 => self.shift_to_right(decoded_operation.x, decoded_operation.y, false),
                0x7 => self.subtract_xregister_from_yregister(decoded_operation.x, decoded_operation.y),
                0xE => self.shift_to_left(decoded_operation.x, decoded_operation.y, false),
                _ => {}
            },
            0x9 => match decoded_operation.n {
                0x0 => self.skip_if_regs_not_equal(decoded_operation.x, decoded_operation.y),
                _ => {}
            },
            0xA => self.set_index_register(decoded_operation.nnn),
            0xD => self.display(
                decoded_operation.x,
                decoded_operation.y,
                decoded_operation.n,
            ),
            _ => {}
        }
    }

    /// Decode the instruction to find out what the emulator should do
    fn decode(&mut self) -> Opcode {
        let instruction: u16 = self.fetch();
        Opcode::decode(instruction)
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
        self.pc %= RAM_SIZE as u16;
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
        self.pc = self.stack.pop().unwrap();
    }

    /// Set register `reg` to `value`
    fn set_register_to_val(&mut self, reg: u8, value: u8) {
        self.variable_registers[reg as usize] = value;
    }

    /// Add `value` to register `reg`
    /// Do not set carry flag on overflow
    fn add_val_to_register(&mut self, reg: u8, value: u8) {
        let i: usize = reg as usize;
        self.variable_registers[i] = self.variable_registers[i].wrapping_add(value)
    }

    /// Set index register `i` to 12-bit value (represented using u16)
    fn set_index_register(&mut self, value: u16) {
        self.i = value
    }

    /// Draw an N pixels tall sprite from the memory location that the index register
    /// is holding to the screen, at the X coordinate in `x_reg` and the Y coordinate in `y_reg`.
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

    /// Check if the value in `reg_num` is equal to `value` and increments PC by 2
    /// if that's the case
    fn skip_if_equal(&mut self, reg_num: u8, value: u8) {
        if self.variable_registers[reg_num as usize] == value {
            self.pc += 2;
        }
    }

    /// Check if the value in `reg_num` is not equal to `value` and increments PC by 2
    /// if that's the case
    fn skip_if_not_equal(&mut self, reg_num: u8, value: u8) {
        if self.variable_registers[reg_num as usize] != value {
            self.pc += 2;
        }
    }

    /// Check if the value in `x_reg` is equal to the value in `y_reg` and
    /// increments PC by 2 if that's the case
    fn skip_if_regs_equal(&mut self, x_reg: u8, y_reg: u8) {
        if self.variable_registers[x_reg as usize] == self.variable_registers[y_reg as usize] {
            self.pc += 2;
        }
    }

    /// Check if the value in `x_reg` is not equal to the value in `y_reg` and
    /// increments PC by 2 if that's the case
    fn skip_if_regs_not_equal(&mut self, x_reg: u8, y_reg: u8) {
        if self.variable_registers[x_reg as usize] != self.variable_registers[y_reg as usize] {
            self.pc += 2;
        }
    }

    /// Sets register `x_reg` to the value of register `y_reg`
    fn set_register_to_register(&mut self, x_reg: u8, y_reg: u8) {
        self.variable_registers[x_reg as usize] = self.variable_registers[y_reg as usize];
    }

    /// Sets register `x_reg` to the result of a bitwise OR between the values in
    /// registers `x_reg` and `y_reg`
    fn bitwise_or(&mut self, x_reg: u8, y_reg: u8) {
        self.variable_registers[x_reg as usize] |= self.variable_registers[y_reg as usize];
    }

    /// Sets register `x_reg` to the result of a bitwise AND between the values in
    /// registers `x_reg` and `y_reg`
    fn bitwise_and(&mut self, x_reg: u8, y_reg: u8) {
        self.variable_registers[x_reg as usize] &= self.variable_registers[y_reg as usize];
    }

    /// Sets register `x_reg` to the result of a bitwise XOR between the values in
    /// registers `x_reg` and `y_reg`
    fn bitwise_xor(&mut self, x_reg: u8, y_reg: u8) {
        self.variable_registers[x_reg as usize] ^= self.variable_registers[y_reg as usize];
    }

    /// Sets register `x_reg` to the result of adding the value of `y_reg` to it
    /// If overflow occurs, sets the flag register to 1, otherwise sets it to 0
    fn add_register_to_register(&mut self, x_reg: u8, y_reg: u8) {
        let mut overflow: bool = false;
        (self.variable_registers[x_reg as usize], overflow) = self.variable_registers
            [x_reg as usize]
            .overflowing_add(self.variable_registers[y_reg as usize]);

        self.variable_registers[15] = if overflow { 1 } else { 0 }
    }

    /// Sets register `x_reg` to the result of subtracting the value of `y_reg` from it
    /// If overflow occurs, sets the flag register to 0, otherwise sets it to 1
    fn subtract_yregister_from_xregister(&mut self, x_reg: u8, y_reg: u8) {
        let mut overflow: bool = false;
        (self.variable_registers[x_reg as usize], overflow) = self.variable_registers
            [x_reg as usize]
            .overflowing_sub(self.variable_registers[y_reg as usize]);

        self.variable_registers[15] = if overflow { 0 } else { 1 }
    }

    /// Sets register `x_reg` to the result of subtracting it from the value of `y_reg`
    /// If overflow occurs, sets the flag register to 0, otherwise sets it to 1
    fn subtract_xregister_from_yregister(&mut self, x_reg: u8, y_reg: u8) {
        let mut overflow: bool = false;
        (self.variable_registers[x_reg as usize], overflow) = self.variable_registers
            [y_reg as usize]
            .overflowing_sub(self.variable_registers[x_reg as usize]);

        self.variable_registers[15] = if overflow { 0 } else { 1 }
    }

    /// Shifts the value in `x_reg` one to the right. If `modern_instructions` is false,
    /// then `y_reg` the value of `x_reg` becomes the value of `y_reg` before shifting,
    /// making it so it's the shifted value of `y_reg` that exists in `x_reg`.
    /// Sets the flag register to the value of the bit that was shifted out.
    fn shift_to_right(&mut self, x_reg: u8, y_reg: u8, modern_instructions: bool) {
        if !modern_instructions {
            self.variable_registers[x_reg as usize] = self.variable_registers[y_reg as usize]
        }
        self.variable_registers[15] = self.variable_registers[x_reg as usize] & 1;
        self.variable_registers[x_reg as usize] >>= 1;
    }

    /// Shifts the value in `x_reg` one to the left. If `modern_instructions` is false,
    /// then `y_reg` the value of `x_reg` becomes the value of `y_reg` before shifting,
    /// making it so it's the shifted value of `y_reg` that exists in `x_reg`.
    /// Sets the flag register to the value of the bit that was shifted out.
    fn shift_to_left(&mut self, x_reg: u8, y_reg: u8, modern_instructions: bool) {
        if !modern_instructions {
            self.variable_registers[x_reg as usize] = self.variable_registers[y_reg as usize]
        }
        self.variable_registers[15] = (self.variable_registers[x_reg as usize] >> 7) & 1;
        self.variable_registers[x_reg as usize] <<= 1;
    }
}
