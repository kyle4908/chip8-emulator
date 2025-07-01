use log::{debug, error, info, warn};
use std::fs;

use std::time::{Duration, Instant};

use crate::keypad::{Keypad, NUM_KEYS};
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

pub struct Emulator {
    ram: [u8; RAM_SIZE],
    screen: [[bool; SCREEN_WIDTH]; SCREEN_HEIGHT], // bool because pixels can be either black or white
    pc: u16, // program counter, points to current instruction in memory, memory addresses are 16 bits
    i: u16,  // index register, used to point to locations in memory
    stack: Vec<u16>, // stack for addresses
    // stack_pointer: u16, // points to the current index we are at on the "stack"
    delay_timer: u8,
    sound_timer: u8,
    variable_registers: [u8; NUM_VARIABLE_REGISTERS],
    pub keypad: Keypad,
    redraw_required: bool, // flag indicating a change to the screen was made
    use_y_on_shift: bool,
    use_x_on_jump: bool,
    last_timer_update: Instant,
}

const START_ADDR: u16 = 0x200;
// CHIP8 programs are supposed to be loaded into memory after address 200

impl Emulator {
    pub fn new(use_y_on_shift: bool, use_x_on_jump: bool) -> Self {
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
            keypad: Keypad::new(),
            redraw_required: false,
            use_y_on_shift,
            use_x_on_jump,
            last_timer_update: Instant::now(),
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

    /// Handles updating of sound, and delay timer 60 times per second
    fn handle_timers(&mut self) {
        let now = Instant::now();
        let timer_update_delta = Duration::from_secs_f64(1.0 / 60.0);
        if now.duration_since(self.last_timer_update) >= timer_update_delta {
            self.delay_timer = self.delay_timer.saturating_sub(1);
            self.sound_timer = self.sound_timer.saturating_sub(1);
            self.last_timer_update = now;
        }
    }

    /// Returns the current state of the sound timer
    pub fn sound_timer(&self) -> &u8 {
        &self.sound_timer
    }

    /// Returns the current state of the screen
    pub fn screen(&self) -> &[[bool; SCREEN_WIDTH]; SCREEN_HEIGHT] {
        &self.screen
    }

    /// Execute the instruction and do what it tells you
    pub fn execute(&mut self) {
        let decoded_operation: Opcode = self.decode();
        debug!("Opcode decoded as {:?}", decoded_operation);
        debug!("Current state of RAM {:?}", self.ram);
        debug!("Current state of Registers {:?}", self.variable_registers);

        self.handle_timers();

        match decoded_operation.category {
            0x0 => match decoded_operation.nnn {
                0x0E0 => self.clear_screen(),
                0x0EE => self.subroutine_exit(),
                _ => warn_unknown_operation(decoded_operation),
            },
            0x1 => self.jump(decoded_operation.nnn),
            0x2 => self.subroutine_call(decoded_operation.nnn),
            0x3 => self.skip_if_equal(decoded_operation.x, decoded_operation.nn),
            0x4 => self.skip_if_not_equal(decoded_operation.x, decoded_operation.nn),
            0x5 => match decoded_operation.n {
                0x0 => self.skip_if_regs_equal(decoded_operation.x, decoded_operation.y),
                _ => warn_unknown_operation(decoded_operation),
            },
            0x6 => self.set_register_to_val(decoded_operation.x, decoded_operation.nn),
            0x7 => self.add_val_to_register(decoded_operation.x, decoded_operation.nn),
            0x8 => match decoded_operation.n {
                0x0 => self.set_register_to_register(decoded_operation.x, decoded_operation.y),
                0x1 => self.bitwise_or(decoded_operation.x, decoded_operation.y),
                0x2 => self.bitwise_and(decoded_operation.x, decoded_operation.y),
                0x3 => self.bitwise_xor(decoded_operation.x, decoded_operation.y),
                0x4 => self.add_register_to_register(decoded_operation.x, decoded_operation.y),
                0x5 => {
                    self.subtract_yregister_from_xregister(decoded_operation.x, decoded_operation.y)
                }
                0x6 => self.shift_to_right(decoded_operation.x, decoded_operation.y),
                0x7 => {
                    self.subtract_xregister_from_yregister(decoded_operation.x, decoded_operation.y)
                }
                0xE => self.shift_to_left(decoded_operation.x, decoded_operation.y),
                _ => warn_unknown_operation(decoded_operation),
            },
            0x9 => match decoded_operation.n {
                0x0 => self.skip_if_regs_not_equal(decoded_operation.x, decoded_operation.y),
                _ => warn_unknown_operation(decoded_operation),
            },
            0xA => self.set_index_register(decoded_operation.nnn),
            0xB => self.jump_with_offset(decoded_operation.x, decoded_operation.nnn),
            0xC => self.random(decoded_operation.x, decoded_operation.nn),
            0xD => self.display(
                decoded_operation.x,
                decoded_operation.y,
                decoded_operation.n,
            ),
            0xE => match decoded_operation.nn {
                0x9E => self.skip_if_key_pressed(decoded_operation.x),
                0xA1 => self.skip_if_key_not_pressed(decoded_operation.x),
                _ => warn_unknown_operation(decoded_operation),
            },
            0xF => match decoded_operation.nn {
                0x07 => self.set_register_to_delay_timer(decoded_operation.x),
                0x0A => self.block_and_wait_for_key(decoded_operation.x),
                0x15 => self.set_delay_timer_to_register_value(decoded_operation.x),
                0x18 => self.set_sound_timer_to_register_value(decoded_operation.x),
                0x1E => self.add_register_to_index_register(decoded_operation.x),
                _ => warn_unknown_operation(decoded_operation),
            },
            _ => warn_unknown_operation(decoded_operation),
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

        debug!("Fetched instruction: {:#X}", instruction);
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
        debug!("Clearing screen");
        for i in 0..SCREEN_HEIGHT {
            for j in 0..SCREEN_WIDTH {
                self.screen[i][j] = false;
            }
        }
        self.redraw_required = true;
    }

    /// Set the PC counter to `memory_location` which is 12-bit, despite using u16 to represent it
    fn jump(&mut self, memory_location: u16) {
        debug!("Jumping to {:#X}", memory_location);
        self.pc = memory_location
    }

    /// Set the PC counter to `memory_location` which is 12-bit, despite using u16 to represent it
    /// and push the current PC to the stack, so the subroutine can return later
    fn subroutine_call(&mut self, memory_location: u16) {
        debug!("Subroutine call to {:#X}", memory_location);
        self.stack.push(self.pc);
        self.pc = memory_location;
    }

    /// Pop the last instruction from the stack and set the PC to it.
    /// used to return from a subroutine
    fn subroutine_exit(&mut self) {
        debug!("Exiting subroutine");
        if let Some(exit_location) = self.stack.pop() {
            self.pc = exit_location
        } else {
            error!("No memory location found in stack, to return to");
            panic!("Attempted to exit subroutine with nowhere to return to")
        }
    }

    /// Set register `reg` to `value`
    fn set_register_to_val(&mut self, reg: u8, value: u8) {
        debug!("Setting register {} to value {}", reg, value);
        self.variable_registers[reg as usize] = value;
    }

    /// Add `value` to register `reg`
    /// Do not set carry flag on overflow
    fn add_val_to_register(&mut self, reg: u8, value: u8) {
        debug!("Adding value {} to register {}", value, reg);
        let i: usize = reg as usize;
        self.variable_registers[i] = self.variable_registers[i].wrapping_add(value)
    }

    /// Set index register `i` to 12-bit value (represented using u16)
    fn set_index_register(&mut self, value: u16) {
        debug!("Setting index register to value {:#X}", value);
        self.i = value
    }

    /// Draw an N pixels tall sprite from the memory location that the index register
    /// is holding to the screen, at the X coordinate in `x_reg` and the Y coordinate in `y_reg`.
    /// All the pixels that are “on” in the sprite will flip the pixels on the screen that it is
    /// drawn to (from left to right, from most to least significant bit). If any pixels on the
    /// screen were turned “off” by this, the VF flag register is set to 1. Otherwise, it’s set to 0.
    fn display(&mut self, x_reg: u8, y_reg: u8, sprite_height: u8) {
        debug!(
            "Drawing {} pixel tall sprite, using X=V[{}], Y=V[{}]",
            sprite_height, y_reg, x_reg
        );
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
        debug!("Skipping if register {} equals {}", reg_num, value);
        if self.variable_registers[reg_num as usize] == value {
            self.pc += 2;
        }
    }

    /// Check if the value in `reg_num` is not equal to `value` and increments PC by 2
    /// if that's the case
    fn skip_if_not_equal(&mut self, reg_num: u8, value: u8) {
        debug!("Skipping if register {} does not equal {}", reg_num, value);
        if self.variable_registers[reg_num as usize] != value {
            self.pc += 2;
        }
    }

    /// Check if the value in `x_reg` is equal to the value in `y_reg` and
    /// increments PC by 2 if that's the case
    fn skip_if_regs_equal(&mut self, x_reg: u8, y_reg: u8) {
        debug!("Skipping if register {} equals register {}", x_reg, y_reg);
        if self.variable_registers[x_reg as usize] == self.variable_registers[y_reg as usize] {
            self.pc += 2;
        }
    }

    /// Check if the value in `x_reg` is not equal to the value in `y_reg` and
    /// increments PC by 2 if that's the case
    fn skip_if_regs_not_equal(&mut self, x_reg: u8, y_reg: u8) {
        debug!(
            "Skipping if register {} does not equal register {}",
            x_reg, y_reg
        );
        if self.variable_registers[x_reg as usize] != self.variable_registers[y_reg as usize] {
            self.pc += 2;
        }
    }

    /// Sets register `x_reg` to the value of register `y_reg`
    fn set_register_to_register(&mut self, x_reg: u8, y_reg: u8) {
        debug!(
            "Setting if register {} to value of register {}",
            x_reg, y_reg
        );
        self.variable_registers[x_reg as usize] = self.variable_registers[y_reg as usize];
    }

    /// Sets register `x_reg` to the result of a bitwise OR between the values in
    /// registers `x_reg` and `y_reg`
    fn bitwise_or(&mut self, x_reg: u8, y_reg: u8) {
        debug!("Bitwise OR of register {} and {}", x_reg, y_reg);
        self.variable_registers[x_reg as usize] |= self.variable_registers[y_reg as usize];
    }

    /// Sets register `x_reg` to the result of a bitwise AND between the values in
    /// registers `x_reg` and `y_reg`
    fn bitwise_and(&mut self, x_reg: u8, y_reg: u8) {
        debug!("Bitwise AND of register {} and {}", x_reg, y_reg);
        self.variable_registers[x_reg as usize] &= self.variable_registers[y_reg as usize];
    }

    /// Sets register `x_reg` to the result of a bitwise XOR between the values in
    /// registers `x_reg` and `y_reg`
    fn bitwise_xor(&mut self, x_reg: u8, y_reg: u8) {
        debug!("Bitwise XOR of register {} and {}", x_reg, y_reg);
        self.variable_registers[x_reg as usize] ^= self.variable_registers[y_reg as usize];
    }

    /// Sets register `x_reg` to the result of adding the value of `y_reg` to it
    /// If overflow occurs, sets the flag register to 1, otherwise sets it to 0
    fn add_register_to_register(&mut self, x_reg: u8, y_reg: u8) {
        debug!("Adding value of register {} to register {}", y_reg, x_reg);
        let (result, overflow) = self.variable_registers[x_reg as usize]
            .overflowing_add(self.variable_registers[y_reg as usize]);
        self.variable_registers[x_reg as usize] = result;
        self.variable_registers[15] = if overflow { 1 } else { 0 }
    }

    /// Sets register `x_reg` to the result of subtracting the value of `y_reg` from it
    /// If overflow occurs, sets the flag register to 0, otherwise sets it to 1
    fn subtract_yregister_from_xregister(&mut self, x_reg: u8, y_reg: u8) {
        debug!("Subtracting value of register {} from {}", y_reg, x_reg);
        let (result, overflow) = self.variable_registers[x_reg as usize]
            .overflowing_sub(self.variable_registers[y_reg as usize]);
        self.variable_registers[x_reg as usize] = result;
        self.variable_registers[15] = if overflow { 0 } else { 1 }
    }

    /// Sets register `x_reg` to the result of subtracting it from the value of `y_reg`
    /// If overflow occurs, sets the flag register to 0, otherwise sets it to 1
    fn subtract_xregister_from_yregister(&mut self, x_reg: u8, y_reg: u8) {
        debug!("Subtracting value of register {} from {}", x_reg, y_reg);
        let (result, overflow) = self.variable_registers[y_reg as usize]
            .overflowing_sub(self.variable_registers[x_reg as usize]);
        self.variable_registers[x_reg as usize] = result;
        self.variable_registers[15] = if overflow { 0 } else { 1 }
    }

    /// Shifts the value in `x_reg` one to the right. If `use_y_on_shift` is true,
    /// then `y_reg` the value of `x_reg` becomes the value of `y_reg` before shifting,
    /// making it so it's the shifted value of `y_reg` that exists in `x_reg`.
    /// Sets the flag register to the value of the bit that was shifted out.
    fn shift_to_right(&mut self, x_reg: u8, y_reg: u8) {
        debug!("Shifting value of register to the right");
        if self.use_y_on_shift {
            self.variable_registers[x_reg as usize] = self.variable_registers[y_reg as usize]
        }
        self.variable_registers[15] = self.variable_registers[x_reg as usize] & 1;
        self.variable_registers[x_reg as usize] >>= 1;
    }

    /// Shifts the value in `x_reg` one to the left. If `use_y_on_shift` is true,
    /// then `y_reg` the value of `x_reg` becomes the value of `y_reg` before shifting,
    /// making it so it's the shifted value of `y_reg` that exists in `x_reg`.
    /// Sets the flag register to the value of the bit that was shifted out.
    fn shift_to_left(&mut self, x_reg: u8, y_reg: u8) {
        debug!("Shifting value of register to the left");
        if self.use_y_on_shift {
            self.variable_registers[x_reg as usize] = self.variable_registers[y_reg as usize]
        }
        self.variable_registers[15] = (self.variable_registers[x_reg as usize] >> 7) & 1;
        self.variable_registers[x_reg as usize] <<= 1;
    }

    /// Jump to memory location of the Register 0 plus the `offset`.
    /// If `use_x_on_jump` is true, then uses Register X instead of Register 0.
    fn jump_with_offset(&mut self, x_reg: u8, offset: u16) {
        debug!("Jumping with offset {:#X}", offset);
        self.pc = if self.use_x_on_jump {
            self.variable_registers[x_reg as usize] as u16 + offset
        } else {
            self.variable_registers[0] as u16 + offset
        }
    }

    /// Generate a random u8 number, binary AND it with `value` and put the result into
    /// register `x_reg`
    fn random(&mut self, x_reg: u8, value: u8) {
        debug!("Generating random number");
        self.variable_registers[x_reg as usize] = rand::random_range(0..=255) & value;
    }

    /// Skip one instruction (increment PC by 2) if the key corresponding to the value in
    /// register `x_reg` is pressed
    fn skip_if_key_pressed(&mut self, reg: u8) {
        debug!(
            "Skipping next instruction if key in register {} is pressed",
            reg
        );
        if self.keypad.get_keys()[self.variable_registers[reg as usize] as usize] {
            self.pc += 2;
        }
    }

    /// Skip one instruction (increment PC by 2) if the key corresponding to the value in
    /// register `x_reg` is not pressed
    fn skip_if_key_not_pressed(&mut self, reg: u8) {
        debug!(
            "Skipping next instruction if key in register {} is not pressed",
            reg
        );
        if !self.keypad.get_keys()[self.variable_registers[reg as usize] as usize] {
            self.pc += 2;
        }
    }

    /// Sets value of register `reg` to delay timer value
    fn set_register_to_delay_timer(&mut self, reg: u8) {
        debug!(
            "Setting register {} to delay timer value {}",
            reg, self.delay_timer
        );
        self.variable_registers[reg as usize] = self.delay_timer;
    }

    /// Sets value delay timer to value in register `reg`
    fn set_delay_timer_to_register_value(&mut self, reg: u8) {
        debug!("Setting delay timer to value in register {}", reg);
        self.delay_timer = self.variable_registers[reg as usize];
    }

    /// Sets value sound timer to value in register `reg`
    fn set_sound_timer_to_register_value(&mut self, reg: u8) {
        debug!("Setting sound timer to value in register {}", reg);
        self.sound_timer = self.variable_registers[reg as usize];
    }

    /// Adds value of register `reg` to index register and sets
    /// overflow flag if overflow occurs
    /// see note here: https://tobiasvl.github.io/blog/write-a-chip-8-emulator/#fx1e-add-to-index
    fn add_register_to_index_register(&mut self, reg: u8) {
        debug!("Adding value of register {} to index register", reg);
        let result: u16 = self.i + self.variable_registers[reg as usize] as u16;
        if result > 0xFFF {
            self.variable_registers[15] = 1;
            self.i = result - 0x1000
        } else {
            self.i = result
        }
    }

    /// Stop executing instructions until a key is pressed
    /// when a key is pressed, put its value into the register `reg`
    fn block_and_wait_for_key(&mut self, reg: u8) {
        info!("Waiting for a key press, to put into register {}", reg);
        for i in 0..NUM_KEYS {
            if self.keypad.get_keys()[i] {
                self.variable_registers[i] = i as u8;
                return;
            }
        }
        self.pc -= 2; // Since PC was incremented on fetch, decrementing to simulate blocking
    }
}

fn warn_unknown_operation(operation: Opcode) {
    warn!("Unknown Operation {:?}", operation);
}
