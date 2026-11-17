
pub const CHIP8_FONTSET: [u8; 80] = [
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

pub struct Cpu {
    pub memory: [u8; 4096],
    pub v: [u8; 16],
    pub i: u16,
    pub pc: u16,
    pub stack: [u16; 16],
    pub sp: usize,
    pub delay_timer: u8,
    pub sound_timer: u8,
    pub display: [bool; 64 * 32],
    pub keypad: [bool; 16],
}
impl Cpu {
    pub fn new() -> Self {
        let mut memory = [0; 4096];
        memory[0x050..0x0A0].copy_from_slice(&CHIP8_FONTSET);
        Self {
            memory,
            v: [0; 16],
            i: 0,
            pc: 0x200,
            stack: [0; 16],
            sp: 0,
            delay_timer: 0,
            sound_timer: 0,
            display: [false; 64 * 32],
            keypad: [false; 16],
        }
    }
    pub fn load_rom(&mut self, data: &[u8]) {
        let start = 0x200;
        let end = start + data.len();
        self.memory[start..end].copy_from_slice(data);
    }
    pub fn tick(&mut self) {
        let hi = self.memory[self.pc as usize] as u16;
        let lo = self.memory[(self.pc + 1) as usize] as u16; // склейка двух байтов
        let opcode:u16 = (hi << 8) | lo;

        self.pc += 2;

        let digit1 = (opcode & 0xF000) >> 12;
        let digit2 = (opcode & 0x0F00) >> 8;
        let digit3 = (opcode & 0x00F0) >> 4;
        let digit4 = opcode & 0x000F;

        match (digit1, digit2, digit3, digit4) {
            (0, 0, 0, 0) => return,//NOP
            (0, 0, 0xE, 0) => self.display = [false; 64 * 32],

            (1, _, _, _) => {
                let nnn = opcode & 0x0FFF;
                self.pc = nnn
            },

            (2, _, _, _) =>{ //call
                self.stack[self.sp] = self.pc;
                self.sp += 1;
                self.pc = opcode & 0x0FFF;
            }
            (0, 0, 0xE, 0xE) => {
                self.sp-=1;
                self.pc = self.stack[self.sp];
            }

            (6, x, _, _) => {
                let nn = (opcode & 0x00FF) as u8;
                self.v[x as usize] = nn;
            }
            (7, x, _, _) => {
                let nn = (opcode & 0x00FF) as u8;
                self.v[x as usize] = self.v[x as usize].wrapping_add(nn);
            }

            _=> unimplemented!("неизвестный опкод: {:#06X}", opcode),

        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_tick(){

        let mut cpu = Cpu::new();
        cpu.load_rom(&[0x62, 0x0A, 0x72, 0x05, 0x13, 0x00]);
        cpu.tick();
        cpu.tick();
        cpu.tick();
        assert_eq!(cpu.pc, 0x300);
        assert_eq!(cpu.v[2], 15);
    }
    #[test]
    fn test_subroutine_call_and_ret(){
        let mut cpu = Cpu::new();
        assert_eq!(cpu.memory[0x050], 0xF0);
        cpu.load_rom(&[0x22, 0x04, 0x60, 0x01, 0x61, 0x05, 0x00, 0xEE]);

        cpu.tick();
        cpu.tick();
        cpu.tick();
        cpu.tick();
        assert_eq!(cpu.v[1], 5);
        assert_eq!(cpu.v[0], 1);
        assert_eq!(cpu.sp, 0);

    }

}
