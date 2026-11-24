/// Встроенный набор шрифтов CHIP-8 (16 символов 0–F, по 5 байт на каждый символ).
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
/// Виртуальный процессор CHIP-8: память 4 КБ, регистры V0–VF, стек, таймеры, экран 64x32 и клавиатура.
pub struct Cpu {
    /// Оперативная память 4 КБ (0x000–0x1FF: система и шрифты, 0x200–0xFFF: ROM)
    pub memory: [u8; 4096],
    /// Регистры общего назначения V0–VF (VF используется как регистр флагов).
    pub v: [u8; 16],
    /// 16-битный индексный регистр I для хранения адресов памяти.
    pub i: u16,
    /// Счётчик команд (Program Counter), указывает на текущую инструкцию.
    pub pc: u16,
    /// Стек вызовов (до 16 вложенных подпрограмм).
    pub stack: [u16; 16],
    /// Указатель вершины стека (Stack Pointer).
    pub sp: usize,
    /// Таймер задержки (уменьшается с частотой 60 Гц).
    pub delay_timer: u8,
    /// Звуковой таймер (подаёт сигнал, пока > 0; уменьшается с частотой 60 Гц).
    pub sound_timer: u8,
    /// Монохромный видеобуфер 64×32 пикселя (true — пиксель горит).
    pub display: [bool; 64 * 32],
    /// Состояние 16-клавишной клавиатуры (true — клавиша нажата).
    pub keypad: [bool; 16],
    seed: u32
}
impl Cpu {
    /// Создаёт новый экземпляр CPU: обнуляет состояние, ставит PC на 0x200 и загружает шрифты в 0x050..0x0A0.
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
            seed: 0xDEADBEEF
        }
    }
    /// Генератор псевдослучайных байтов (Xorshift32) без зависимости от std.
    fn rand_byte(&mut self) -> u8 {
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.seed = x;
        x as u8
    }
    /// Копирует байты ROM-файла в память начиная со стандартного адреса 0x200.
    pub fn load_rom(&mut self, data: &[u8]) {
        let start = 0x200;
        let end = start + data.len();
        self.memory[start..end].copy_from_slice(data);
    }
    /// Выполняет один цикл fetch-decode-execute: читает 2-байтный опкод, сдвигает PC и исполняет инструкцию.
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
            // 0000: NOP (пустая операция)
            (0, 0, 0, 0) => return,
            // 00E0: CLS — очистить экран
            (0, 0, 0xE, 0) => self.display = [false; 64 * 32],
            // 1NNN: JP addr — безусловный переход на адрес NNN
            (1, _, _, _) => {
                let nnn = opcode & 0x0FFF;
                self.pc = nnn
            },
            // 2NNN: CALL addr — сохранить PC в стек и перейти к подпрограмме NNN
            (2, _, _, _) => { //call
                self.stack[self.sp] = self.pc;
                self.sp += 1;
                self.pc = opcode & 0x0FFF;
            }
            // 00EE: RET — возврат из подпрограммы (достать PC из стека)
            (0, 0, 0xE, 0xE) => {
                self.sp -= 1;
                self.pc = self.stack[self.sp];
            }
            // 3XNN: SE Vx, byte — пропустить следующую инструкцию, если VX == NN
            (3, x, _, _) => {
                let nn = (opcode & 0x00FF) as u8;
                if self.v[x as usize] == nn {
                    self.pc += 2;
                }
            }
            // 4XNN: SNE Vx, byte — пропустить следующую инструкцию, если VX != NN
            (4, x, _, _) => {
                let nn = (opcode & 0x00FF) as u8;
                if self.v[x as usize] != nn {
                    self.pc += 2;
                }
            }
            // 5XY0: SE Vx, Vy — пропустить следующую инструкцию, если VX == VY
            (5, x, y, 0) => {
                if self.v[x as usize] == self.v[y as usize] {
                    self.pc += 2;
                }
            }
            // 9XY0: SNE Vx, Vy — пропустить следующую инструкцию, если VX != VY
            (9, x, y, 0) => {
                if self.v[x as usize] != self.v[y as usize] {
                    self.pc += 2;
                }
            }
            // ANNN: LD I, addr — записать адрес NNN в индексный регистр I
            (0xA, _, _, _) => {
                let nnn = opcode & 0x0FFF;
                self.i = nnn;
            }
            // BNNN: JP V0, addr — переход на адрес NNN + V0
            (0xB, _, _, _) => {
                let nnn = opcode & 0x0FFF;
                self.pc = nnn + self.v[0] as u16;
            }
            // CXNN: RND Vx, byte — VX = random_byte & NN
            (0xC, x, _, _) => {
                let nn = (opcode & 0x00FF) as u8;
                let r = self.rand_byte();
                self.v[x as usize] = r & nn;
            }
            // 6XNN: LD Vx, byte — записать константу NN в регистр VX
            (6, x, _, _) => {
                let nn = (opcode & 0x00FF) as u8;
                self.v[x as usize] = nn;
            }
            // 7XNN: ADD Vx, byte — прибавить NN к VX без изменения флага переноса VF
            (7, x, _, _) => {
                let nn = (opcode & 0x00FF) as u8;
                self.v[x as usize] = self.v[x as usize].wrapping_add(nn);
            }
            // 8XY0: LD Vx, Vy — присвоить VX = VY
            (8, x, y, 0) => {
                self.v[x as usize] = self.v[y as usize];
            }
            // 8XY1: OR Vx, Vy — побитовое ИЛИ: VX |= VY
            (8, x, y, 1) => {
                self.v[x as usize] |= self.v[y as usize];
            }
            // 8XY2: AND Vx, Vy — побитовое И: VX &= VY
            (8, x, y, 2) => {
                self.v[x as usize] &= self.v[y as usize];
            }
            // 8XY3: XOR Vx, Vy — побитовое исключающее ИЛИ: VX ^= VY
            (8, x, y, 3) => {
                self.v[x as usize] ^= self.v[y as usize];
            }
            // 8XY4: ADD Vx, Vy — VX += VY; VF = 1 при переполнении (> 255), иначе 0
            (8, x, y, 4) => {
                let (sum, did_overflow) = self.v[x as usize].overflowing_add(self.v[y as usize]);
                self.v[x as usize] = sum;
                self.v[0xF] = if did_overflow { 1 } else { 0 };
            }
            // 8XY5: SUB Vx, Vy — VX -= VY; VF = 1 если не было заёма (VX >= VY), иначе 0
            (8, x, y, 5) => {
                let (diff, overflow) = self.v[x as usize].overflowing_sub(self.v[y as usize]);
                self.v[x as usize] = diff;
                self.v[0xF] = if overflow { 0 } else { 1 };
            }
            // 8XY7: SUBN Vx, Vy — VX = VY - VX; VF = 1 если не было заёма (VY >= VX), иначе 0
            (8, x, y, 7) => {
                let (diff, overflow) = self.v[y as usize].overflowing_sub(self.v[x as usize]);
                self.v[x as usize] = diff;
                self.v[0xF] = if overflow { 0 } else { 1 };
            }
            // 8XY6: SHR Vx — сдвиг VX вправо на 1 бит; VF = выдвинутый младший бит
            (8, x, _, 6) => {
                let last = self.v[x as usize] & 1;
                self.v[x as usize] >>= 1;
                self.v[0xF] = last;
            }
            // 8XYE: SHL Vx — сдвиг VX влево на 1 бит; VF = выдвинутый старший бит
            (8, x, _, 0xE) => {
                let last = self.v[x as usize] >> 7 & 1;
                self.v[x as usize] <<= 1;
                self.v[0xF] = last;
            }
            // EX9E: SKP Vx — пропустить следующую инструкцию, если клавиша с номером VX нажата
            (0xE, x, 9, 0xE) => {
                if self.keypad[self.v[x as usize] as usize] {
                    self.pc += 2;
                }
            }
            // EXA1: SKNP Vx — пропустить следующую инструкцию, если клавиша с номером VX НЕ нажата
            (0xE, x, 0xA, 1) => {
                if !(self.keypad[self.v[x as usize] as usize]) {
                    self.pc += 2;
                }
            }
            // FX0A: LD Vx, K — ждать нажатия любой клавиши и сохранить её код в VX (иначе откатить PC)
            (0xF, x, 0, 0xA) => {
                let mut pressed = false;
                for i in 0..16 {
                    if self.keypad[i] == true {
                        self.v[x as usize] = i as u8;
                        pressed = true;
                        break;
                    }
                }
                if !pressed {
                    self.pc -= 2;
                }
            }
            // FX07: LD Vx, DT — записать значение таймера задержки в VX
            (0xF, x, 0, 7) => {
                self.v[x as usize] = self.delay_timer;
            }
            // FX15: LD DT, Vx — установить таймер задержки равным VX
            (0xF, x, 1, 5) => {
                self.delay_timer = self.v[x as usize];
            }
            // FX18: LD ST, Vx — установить звуковой таймер равным VX
            (0xF, x, 1, 8) => {
                self.sound_timer = self.v[x as usize];
            }
            // FX1E: ADD I, Vx — прибавить VX к регистру I
            (0xF, x, 1, 0xE) => {
                self.i += self.v[x as usize] as u16;
            }
            // FX29: LD F, Vx — установить I на адрес спрайта символа VX во встроенном шрифте (0x050 + VX * 5)
            (0xF, x, 2, 9) => {
                self.i = 0x050 + (self.v[x as usize] as u16) * 5;
            }
            // FX33: LD B, Vx — разложить VX на сотни, десятки и единицы (BCD) по адресам I, I+1, I+2
            (0xF, x, 3, 3) => {
                self.memory[self.i as usize] = self.v[x as usize] / 100;
                self.memory[(self.i + 1) as usize] = self.v[x as usize] / 10 % 10;
                self.memory[(self.i + 2) as usize] = self.v[x as usize] % 10;
            }
            // FX55: LD [I], Vx — сохранить регистры V0..=VX в память начиная с адреса I
            (0xF, x, 5, 5) => {
                for idx in 0..=x {
                    self.memory[self.i as usize + idx as usize] = self.v[idx as usize];
                }
            }
            // FX65: LD Vx, [I] — загрузить регистры V0..=VX из памяти начиная с адреса I
            (0xF, x, 6, 5) => {
                for idx in 0..=x {
                    self.v[idx as usize] = self.memory[self.i as usize + idx as usize];
                }
            }
            // DXYN: DRW Vx, Vy, nibble — нарисовать спрайт 8xN из памяти [I] в (VX, VY) через XOR; VF = 1 при коллизии
            (0xD, x, y, _) => {
                let y_coord = self.v[y as usize] as usize % 32;
                let x_coord = self.v[x as usize] as usize % 64;
                self.v[0xF] = 0;
                for row in 0..digit4 as usize {
                    if y_coord + row >= 32 {
                        break;
                    }
                    let sprite = self.memory[(self.i as usize) + row];
                    for i in 0..8 {
                        if x_coord + i >= 64 {
                            break;
                        }
                        if (sprite & (0x80 >> i)) != 0 {
                            let idx = (y_coord + row) * 64 + (x_coord + i);
                            if self.display[idx] {
                                self.v[0xF] = 1;
                            }
                            self.display[idx] ^= true;
                        }
                    }
                }
            }

            _ => unimplemented!("неизвестный опкод: {:#06X}", opcode),
        }
    }
    /// Уменьшает таймеры задержки и звука на 1, если они больше нуля (частота 60 Гц).
    pub fn tick_timer(&mut self){
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // Проверка базового цикла: загрузка константы (6XNN), сложение (7XNN) и переход (1NNN)
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
    // Проверка базового цикла: загрузка константы (6XNN), сложение (7XNN) и переход (1NNN)
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
    #[test]
    // Проверка арифметики с переполнением (8XY4) и вычитания с заёмом (8XY5) + флаг VF
    fn test_arithmetic_and_flags(){
        let mut cpu = Cpu::new();
        cpu.load_rom(&[0x60, 0xC8,  0x61, 0x64, 0x80, 0x14, 0x80, 0x15]);
        cpu.tick();
        cpu.tick();
        cpu.tick();
        assert_eq!(cpu.v[0], 44);
        assert_eq!(cpu.v[0xF], 1);
        cpu.tick();
        assert_eq!(cpu.v[0xF], 0);
    }
    #[test]
    // Проверка отрисовки спрайта (DXYN), стирания через XOR и установки флага коллизии VF
    fn test_draw_and_collision(){
        let mut cpu = Cpu::new();
        cpu.load_rom(&[0xA0, 0x50, 0x60, 0x00, 0x61, 0x00, 0xD0, 0x11, 0xD0, 0x11]);
        cpu.tick();
        cpu.tick();
        cpu.tick();
        cpu.tick();
        assert_eq!(cpu.display[0..4], [true, true, true, true]);
        assert_eq!(cpu.display[4], false);
        assert_eq!(cpu.v[0xF], 0);
        cpu.tick();
        assert_eq!(cpu.display[0..4], [false, false, false, false]);
        assert_eq!(cpu.v[0xF], 1);
    }
    #[test]
    // Проверка разложения числа на сотни, десятки и единицы (BCD, опкод FX33)
    fn test_bcd(){
        let mut cpu = Cpu::new();
        cpu.load_rom(&[0x60, 0xEA, 0xA3, 0x00, 0xF0, 0x33]);
        cpu.tick();
        cpu.tick();
        cpu.tick();
        assert_eq!(cpu.memory[0x300..0x303], [2, 3, 4]);
    }

    // Прогон официального тестового ROM corax89 с выводом экрана 64x32 в консоль
    #[test]
    fn test_core89_rom(){
        let mut cpu = Cpu::new();
        cpu.load_rom(include_bytes!("../roms/test_opcode.ch8"));
        for _ in 0..500{
            cpu.tick();
        }
        for y in 0..32{
            for x in 0..64{
                if cpu.display[y*64+x]{
                    print!("██")
                }
                else {
                    print!("  ")
                }
            }
            println!();
        }
    }

}
