pub struct Nibbles {
    pub category: u8, // The first nibble. Tells you what kind of instruction it is
    pub x: u8, // The second nibble. Used to look up one of the 16 registers (VX) from V0 through VF
    pub y: u8, // The third nibble. Also used to look up one of the 16 registers (VY) from V0 through VF
    pub n: u8, // The fourth nibble. A 4-bit number
    pub nn: u8, // The second byte (third and fourth nibbles). An 8-bit immediate number
    pub nnn: u16, //The second, third and fourth nibbles. A 12-bit immediate memory address
}

impl Nibbles{
    pub fn new(instruction: u16) -> Self {
        Self { 
            category: ((instruction & 0xF000) >> 12) as u8,
            x: ((instruction & 0x0F00) >> 8) as u8, 
            y: ((instruction & 0x00F0) >> 4) as u8, 
            n: (instruction & 0x000F) as u8, 
            nn: (instruction & 0x00FF) as u8, 
            nnn: (instruction & 0x0FFF) 
        }
    }
}
