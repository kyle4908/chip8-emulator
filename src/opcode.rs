pub struct Opcode {
    pub category: u8, // The first 4 bits (bit 1-4). Tells you what kind of instruction it is
    pub x: u8, // The second 4 bits (bit 5-8). Used to look up one of the 16 registers (VX) from V0 through VF
    pub y: u8, // The third 4 bits (bit 9-12). Also used to look up one of the 16 registers (VY) from V0 through VF
    pub n: u8, // The fourth 4 bits (bit 13-16).
    pub nn: u8, // The second byte (bit 9-16). An 8-bit immediate number
    pub nnn: u16, //The last 12 bits of the opcode (bit 5-16). A 12-bit immediate memory address
}

impl Opcode {
    pub fn decode(instruction: u16) -> Self {
        Self {
            category: ((instruction & 0xF000) >> 12) as u8,
            x: ((instruction & 0x0F00) >> 8) as u8,
            y: ((instruction & 0x00F0) >> 4) as u8,
            n: (instruction & 0x000F) as u8,
            nn: (instruction & 0x00FF) as u8,
            nnn: (instruction & 0x0FFF),
        }
    }
}
