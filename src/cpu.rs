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
        Self {
            memory: [0; 4096],
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

}
