pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;
const RAM_SIZE: usize = 4096;

const NUM_V_REGS: usize = 16;
const STACK_SIZE: usize = 16;
const NUM_KEYS: usize = 16;

const START_ADDR: u16 = 0x200;
const FONTSET_SIZE: usize = 80;
const FONTSET: [u8; FONTSET_SIZE] = [
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
    program_counter: u16,
    ram: [u8; RAM_SIZE],
    screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    v_registers: [u8; NUM_V_REGS],
    i_register: u16,
    stack: [u16; STACK_SIZE],
    stack_pointer: u16,
    keys: [bool; NUM_KEYS],
    delay_timer: u8,
    sound_timer: u8,
}

type Digits = (u16, u16, u16, u16);

impl Emulator {
    pub fn new() -> Emulator {
        let mut emulator = Emulator {
            program_counter: START_ADDR,
            ram: [0; RAM_SIZE],
            screen: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            v_registers: [0; NUM_V_REGS],
            i_register: 0,
            stack_pointer: 0,
            stack: [0; STACK_SIZE],
            keys: [false; NUM_KEYS],
            delay_timer: 0,
            sound_timer: 0,
        };

        emulator.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
        emulator
    }

    pub fn load(&mut self, data: &[u8]) {
        let start = START_ADDR as usize;
        let end = (START_ADDR as usize) + data.len();

        self.ram[start..end].copy_from_slice(data);
    }
    pub fn get_display(&self) -> &[bool] {
        &self.screen
    }

    pub fn keypress(&mut self, idx: usize, pressed: bool) {
        assert!(idx < 16);

        self.keys[idx] = pressed;
    }
    fn push(&mut self, val: u16) {
        self.stack[self.stack_pointer as usize] = val;
        self.stack_pointer += 1;
    }

    fn pop(&mut self) -> u16 {
        self.stack_pointer -= 1;
        self.stack[self.stack_pointer as usize]
    }

    pub fn tick(&mut self) {
        // Fetch
        let operation = self.fetch();

        // Decode & Execute
        self.execute(operation);
    }

    pub fn tick_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            if self.sound_timer == 1 {
                // MAKE BEEP
            }
            self.sound_timer -= 1;
        }
    }

    fn fetch(&mut self) -> u16 {
        let higher_byte = self.ram[self.program_counter as usize] as u16;
        let lower_byte = self.ram[(self.program_counter + 1) as usize] as u16;
        let op = (higher_byte << 8) | lower_byte;

        self.program_counter += 2;

        op
    }

    fn execute(&mut self, op: u16) {
        let digit1 = (op & 0xF000) >> 12;
        let digit2 = (op & 0x0F00) >> 8;
        let digit3 = (op & 0x00F0) >> 4;
        let digit4 = op & 0x000F;

        let digits = (digit1, digit2, digit3, digit4);

        // Operations by OpCode
        match digits {
            // 0000 - Nop ::: no-op / do nothing
            (0, 0, 0, 0) => return,
            // 00E0 - CLS ::: clear screen
            (0, 0, 0xE, 0) => self.op_00e0(),
            // 00EE - Return from Subroutine
            (0, 0, 0xE, 0xE) => self.op_00ee(),
            // 1NNN - Jump ::: move program counter to given address
            (1, _, _, _) => self.op_1nnn(op),
            // 2NNN - Call Subroutine
            (2, _, _, _) => self.op_2nnn(op),
            // 3XNN - Skip next if VX == NN ::: if/else, execute line based on condition
            (3, _, _, _) => self.op_3xnn(op, digits),
            // 4XNN - Skip next if VX != NN ::: same as prev but reversed condition
            (4, _, _, _) => self.op_4xnn(op, digits),
            // 5XY0 - Skip next if VX == VY ::: uses 2 v registers, one for each value
            (5, _, _, 0) => self.op_5xy0(digits),
            // 6XNN - VX = NN ::: Assign second-digit v_register to given value
            (6, _, _, _) => self.op_6xnn(op, digits),
            // 7XNN - VX += NN ::: Adds value to VX register
            (7, _, _, _) => self.op_7xnn(op, digits),
            // 8XY0 - VX = VY ::: Assigns value from VY register to VX register
            (8, _, _, 0) => self.op_8xy0(digits),
            // 8XY1 - Bitwise OR operation
            (8, _, _, 1) => self.op_8xy1(digits),
            // 8XY2 - Bitwise AND operation
            (8, _, _, 2) => self.op_8xy2(digits),
            // 8XY3 - Bitwise XOR operation
            (8, _, _, 3) => self.op_8xy3(digits),
            // 8XY4 - VX += VY ::: Store carry flag (1 or 0) in VF (flag) register after add
            (8, _, _, 4) => self.op_8xy4(digits),
            // 8XY5 - VX -= VY ::: Store carry(borrow) flag in VF register after subtraction
            (8, _, _, 5) => self.op_8xy5(digits),
            // 8XY6 - VX >>= 1 ::: Right shift value in VX register, store dropped bit in VF
            (8, _, _, 6) => self.op_8xy6(digits),
            // 8XY7 - VX = VY - VX
            (8, _, _, 7) => self.op_8xy7(digits),
            // 8XYE - VX <<= 1
            (8, _, _, 0xE) => self.op_8xye(digits),
            // 9XY0 - Skip if VX != VY
            (9, _, _, 0) => self.op_9xy0(digits),
            // ANNN - I = NNN ::: Set I-register to nnn value
            (0xA, _, _, _) => self.op_annn(op),
            // BNNN - Jump to V0 + NNN
            (0xB, _, _, _) => self.op_bnnn(op),
            // CXNN - VX = random() & NN
            (0xC, _, _, _) => self.op_cxnn(op, digits),
            // DXYN - Draw Sprite
            (0xD, _, _, _) => self.op_dxyn(digits),
            // EX9E - Skip if Key Pressed
            (0xE, _, 9, 0xE) => self.op_ex9e(digits),
            // EXA1 - Skip if Key Not Presssed
            (0xE, _, 0xA, 1) => self.op_exa1(digits),
            // FX07 - VX = DT
            (0xF, _, 0, 7) => self.op_fx07(digits),
            // FX0A - Wait for Key Press
            (0xF, _, 0, 0xA) => self.op_fx0a(digits),
            // FX15 - DT = VX
            (0xF, _, 1, 5) => self.op_fx15(digits),
            // FX18 - ST = VX
            (0xF, _, 1, 8) => self.op_fx18(digits),
            // FX1E - I += VX
            (0xF, _, 1, 0xE) => self.op_fx1e(digits),
            // FX29 - Set I to Font Address
            (0xF, _, 2, 9) => self.op_fx29(digits),
            // FX33 - I = Binary-Coded Decimal of VX
            (0xF, _, 3, 3) => self.op_fx33(digits),
            // FX55 - Store V0 - VX into I
            (0xF, _, 5, 5) => self.op_fx55(digits),
            // FX65 - Load I into V0 - VX
            (0xF, _, 6, 5) => self.op_fx65(digits),
            // Wildcard
            (_, _, _, _) => unimplemented!("Unimplemented opcode: {}", &op),
        }
    }

    // A Function for each Opcode
    fn op_00e0(&mut self) {
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
    }

    fn op_00ee(&mut self) {
        self.program_counter = self.pop();
    }

    fn op_1nnn(&mut self, op: u16) {
        self.program_counter = op & 0xFFF;
    }

    fn op_2nnn(&mut self, op: u16) {
        self.push(self.program_counter);
        self.program_counter = op & 0xFFF;
    }

    fn op_3xnn(&mut self, op: u16, digits: Digits) {
        let vx = digits.1 as usize;
        let value = (op & 0xFF) as u8;

        if self.v_registers[vx] == value {
            self.program_counter += 2;
        }
    }

    fn op_4xnn(&mut self, op: u16, digits: Digits) {
        let vx = digits.1 as usize;
        let value = (op & 0xFF) as u8;

        if self.v_registers[vx] != value {
            self.program_counter += 2;
        }
    }

    fn op_5xy0(&mut self, digits: Digits) {
        let (vx, vy) = (digits.1 as usize, digits.2 as usize);

        if self.v_registers[vx] == self.v_registers[vy] {
            self.program_counter += 2;
        }
    }

    fn op_6xnn(&mut self, op: u16, digits: Digits) {
        let vx = digits.1 as usize;
        let value = (op & 0xFF0) as u8;

        self.v_registers[vx] = value;
    }

    fn op_7xnn(&mut self, op: u16, digits: Digits) {
        let vx = digits.1 as usize;
        let value = (op & 0xFF) as u8;

        self.v_registers[vx] = self.v_registers[vx].wrapping_add(value);
    }

    fn op_8xy0(&mut self, digits: Digits) {
        let (vx, vy) = (digits.1 as usize, digits.2 as usize);

        self.v_registers[vx] = self.v_registers[vy];
    }

    fn op_8xy1(&mut self, digits: Digits) {
        let (vx, vy) = (digits.1 as usize, digits.2 as usize);

        self.v_registers[vx] |= self.v_registers[vy];
    }

    fn op_8xy2(&mut self, digits: Digits) {
        let (vx, vy) = (digits.1 as usize, digits.2 as usize);

        self.v_registers[vx] &= self.v_registers[vy];
    }

    fn op_8xy3(&mut self, digits: Digits) {
        let (vx, vy) = (digits.1 as usize, digits.2 as usize);

        self.v_registers[vx] ^= self.v_registers[vy];
    }

    fn op_8xy4(&mut self, digits: Digits) {
        let (vx, vy) = (digits.1 as usize, digits.2 as usize);

        let (new_vx, carry) = self.v_registers[vx].overflowing_add(self.v_registers[vy]);
        let new_vf = if carry { 1 } else { 0 };

        self.v_registers[vx] = new_vx;
        self.v_registers[0xF] = new_vf;
    }

    fn op_8xy5(&mut self, digits: Digits) {
        let (vx, vy) = (digits.1 as usize, digits.2 as usize);

        let (new_vx, borrow) = self.v_registers[vx].overflowing_sub(self.v_registers[vy]);
        let new_vf = if borrow { 0 } else { 1 };

        self.v_registers[vx] = new_vx;
        self.v_registers[0xF] = new_vf;
    }

    fn op_8xy6(&mut self, digits: Digits) {
        let (_, d2, _, _) = digits;
        let vx = d2 as usize;

        let lsb = self.v_registers[vx] & 1;

        self.v_registers[vx] >>= 1;
        self.v_registers[0xF] = lsb;
    }

    fn op_8xy7(&mut self, digits: Digits) {
        let (_, d2, d3, _) = digits;
        let (vx, vy) = (d2 as usize, d3 as usize);

        let (new_vx, borrow) = self.v_registers[vy].overflowing_sub(self.v_registers[vx]);
        let new_vf = if borrow { 0 } else { 1 };

        self.v_registers[vx] = new_vx;
        self.v_registers[0xF] = new_vf;
    }

    fn op_8xye(&mut self, digits: Digits) {
        let (_, d2, _, _) = digits;
        let vx = d2 as usize;
        let msb = (self.v_registers[vx] >> 7) & 1;

        self.v_registers[vx] <<= 1;
        self.v_registers[0xF] = msb;
    }

    fn op_9xy0(&mut self, digits: Digits) {
        let (_, d2, d3, _) = digits;
        let (vx, vy) = (d2 as usize, d3 as usize);

        if self.v_registers[vx] != self.v_registers[vy] {
            self.program_counter += 2;
        }
    }

    fn op_annn(&mut self, op: u16) {
        self.i_register = op & 0xFFF;
    }

    fn op_bnnn(&mut self, op: u16) {
        self.program_counter = (self.v_registers[0] as u16) + op & 0xFFF;
    }

    fn op_cxnn(&mut self, op: u16, digits: Digits) {
        let vx = digits.1 as usize;
        let rng: u8 = rand::random();
        self.v_registers[vx] = rng & (op & 0xFF) as u8;
    }

    fn op_dxyn(&mut self, digits: Digits) {
        let (_, d2, d3, d4) = digits;
        let (vx, vy, num_rows) = (d2 as usize, d3 as usize, d4);

        let x_coord = self.v_registers[vx] as u16;
        let y_coord = self.v_registers[vy] as u16;

        let mut flipped = false;

        for y_line in 0..num_rows {
            let address = self.i_register + y_line as u16;
            let pixels = self.ram[address as usize];

            for x_line in 0..8 {
                if (pixels & (0b1000_0000 >> x_line)) != 0 {
                    let x = (x_coord + x_line) as usize % SCREEN_WIDTH;

                    let y = (y_coord + y_line) as usize % SCREEN_HEIGHT;

                    let idx = x + SCREEN_WIDTH * y;

                    flipped |= self.screen[idx];
                    self.screen[idx] ^= true;
                }
            }
        }

        if flipped {
            self.v_registers[0xF] = 1;
        } else {
            self.v_registers[0xF] = 0;
        }
    }

    fn op_ex9e(&mut self, digits: Digits) {
        let x = digits.1 as usize;
        let vx = self.v_registers[x];
        let key = self.keys[vx as usize];

        if key {
            self.program_counter += 2;
        }
    }

    fn op_exa1(&mut self, digits: Digits) {
        let x = digits.1 as usize;
        let vx = self.v_registers[x];
        let key = self.keys[vx as usize];

        if !key {
            self.program_counter += 2;
        }
    }

    fn op_fx07(&mut self, digits: Digits) {
        let x = digits.1 as usize;
        self.v_registers[x] = self.delay_timer;
    }

    fn op_fx0a(&mut self, digits: Digits) {
        let x = digits.1 as usize;
        let mut pressed = false;

        for i in 0..self.keys.len() {
            if self.keys[i] {
                self.v_registers[x] = i as u8;
                pressed = true;
                break;
            }
        }

        if !pressed {
            self.program_counter -= 2;
        }
    }

    fn op_fx15(&mut self, digits: Digits) {
        let x = digits.1 as usize;
        self.delay_timer = self.v_registers[x];
    }

    fn op_fx18(&mut self, digits: Digits) {
        let x = digits.1 as usize;
        self.sound_timer = self.v_registers[x];
    }

    fn op_fx1e(&mut self, digits: Digits) {
        let x = digits.1 as usize;
        let vx = self.v_registers[x] as u16;
        self.i_register = self.i_register.wrapping_add(vx);
    }

    fn op_fx29(&mut self, digits: Digits) {
        let x = digits.1 as usize;
        let c = self.v_registers[x] as u16;
        self.i_register = c * 5;
    }

    fn op_fx33(&mut self, digits: Digits) {
        let x = digits.1 as usize;
        let vx = self.v_registers[x] as f32;

        let hundreds = (vx / 100.0).floor() as u8;
        let tens = ((vx / 10.0) % 10.0).floor() as u8;
        let ones = (vx % 10.0) as u8;

        self.ram[self.i_register as usize] = hundreds;
        self.ram[(self.i_register + 1) as usize] = tens;
        self.ram[(self.i_register + 2) as usize] = ones;
    }

    fn op_fx55(&mut self, digits: Digits) {
        let x = digits.1 as usize;
        let i = self.i_register as usize;

        for idx in 0..=x {
            self.ram[i + idx] = self.v_registers[idx];
        }
    }

    fn op_fx65(&mut self, digits: Digits) {
        let x = digits.1 as usize;
        let i = self.i_register as usize;

        for idx in 0..=x {
            self.v_registers[idx] = self.ram[i + idx];
        }
    }

    pub fn reset(&mut self) {
        self.program_counter = START_ADDR;
        self.ram = [0; RAM_SIZE];
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
        self.v_registers = [0; NUM_V_REGS];
        self.i_register = 0;
        self.stack_pointer = 0;
        self.stack = [0; STACK_SIZE];
        self.keys = [false; NUM_KEYS];
        self.delay_timer = 0;
        self.sound_timer = 0;
        self.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
    }
}
