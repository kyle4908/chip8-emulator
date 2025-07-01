use sdl2::keyboard::Keycode;

const NUM_KEYS: usize = 16;
// CHIP8 usually used on computers with hexidecimal keypads

pub struct Keypad {
    keys: [bool; NUM_KEYS],
}

impl Keypad {
    pub fn new() -> Self {
        Self {
            keys: [false; NUM_KEYS],
        }
    }

    /// Get the current keyboard state
    pub fn get_keys(&self) -> &[bool; NUM_KEYS] {
        &self.keys
    }

    /// Set the key corresponding to the given keycode to true
    pub fn key_down(&mut self, keycode: Keycode) {
        if let Some(index) = Self::key_mapping(keycode) {
            self.keys[index as usize] = true;
        }
    }

    /// Set the key corresponding to the given keycode to false
    pub fn key_up(&mut self, keycode: Keycode) {
        if let Some(index) = Self::key_mapping(keycode) {
            self.keys[index as usize] = false;
        }
    }

    /// Get mapping of computer keyboard key to CHIP8 key
    fn key_mapping(keycode: Keycode) -> Option<u8> {
        match keycode {
            Keycode::NUM_1 => Some(0x1),
            Keycode::NUM_2 => Some(0x2),
            Keycode::NUM_3 => Some(0x3),
            Keycode::NUM_4 => Some(0xC),
            Keycode::Q => Some(0x4),
            Keycode::W => Some(0x5),
            Keycode::E => Some(0x6),
            Keycode::R => Some(0xD),
            Keycode::A => Some(0x7),
            Keycode::S => Some(0x8),
            Keycode::D => Some(0x9),
            Keycode::F => Some(0xE),
            Keycode::Z => Some(0xA),
            Keycode::X => Some(0x0),
            Keycode::C => Some(0xB),
            Keycode::V => Some(0xF),
            _ => None,
        }
    }
}
