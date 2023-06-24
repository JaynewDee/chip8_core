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

        // Operations by OpCode
        match (digit1, digit2, digit3, digit4) {
            // 0000 - Nop ::: no-op / do nothing
            (0, 0, 0, 0) => return,
            // 00E0 - CLS ::: clear screen
            (0, 0, 0xE, 0) => {
                self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
            }
            // 00EE - Return from Subroutine
            (0, 0, 0xE, 0xE) => {
                let ret_address = self.pop();
                self.program_counter = ret_address;
            }
            // 1NNN - Jump ::: move program counter to given address
            (1, _, _, _) => {
                let jump_to = op & 0xFFF;
                self.program_counter = jump_to;
            }
            // 2NNN - Call Subroutine
            (2, _, _, _) => {
                let sub_address = op & 0xFFF;
                self.push(self.program_counter);
                self.program_counter = sub_address;
            }
            // 3XNN - Skip next if VX == NN ::: if/else, execute line based on condition
            (3, _, _, _) => {
                let v_reg_idx = digit2 as usize;
                let value = (op & 0xFF) as u8;

                if self.v_registers[v_reg_idx] == value {
                    self.program_counter += 2;
                }
            }
            // 4XNN - Skip next if VX != NN ::: same as prev but reversed condition
            (4, _, _, _) => {
                let v_reg_idx = digit2 as usize;
                let value = (op & 0xFF) as u8;

                if self.v_registers[v_reg_idx] != value {
                    self.program_counter += 2;
                }
            }
            // 5XY0 - Skip next if VX == VY ::: uses 2 v registers, one for each value
            (5, _, _, 0) => {
                let v_reg1 = digit2 as usize;
                let v_reg2 = digit3 as usize;

                if self.v_registers[v_reg1] == self.v_registers[v_reg2] {
                    self.program_counter += 2;
                }
            }
            // 6XNN - VX = NN ::: Assign second-digit v_register to given value
            (6, _, _, _) => {
                let v_reg = digit2 as usize;
                let value = (op & 0xFF) as u8;

                self.v_registers[v_reg] = value;
            }
            // 7XNN - VX += NN ::: Adds value to VX register
            (7, _, _, _) => {
                let v_reg = digit2 as usize;
                let value = (op & 0xFF) as u8;

                self.v_registers[v_reg] = self.v_registers[v_reg].wrapping_add(value);
            }
            // 8XY0 - VX = VY ::: Assigns value from VY register to VX register
            (8, _, _, 0) => {
                let v_reg_x = digit2 as usize;
                let v_reg_y = digit3 as usize;
                self.v_registers[v_reg_x] = self.v_registers[v_reg_y]
            }
            // 8XY1 - Bitwise OR operation
            (8, _, _, 1) => {
                let v_reg_x = digit2 as usize;
                let v_reg_y = digit3 as usize;

                self.v_registers[v_reg_x] |= self.v_registers[v_reg_y];
            } 
            // 8XY2 - Bitwise AND operation
            (8, _, _, 2) => {
                let v_reg_x = digit2 as usize;
                let v_reg_y = digit3 as usize;

                self.v_registers[v_reg_x] &= self.v_registers[v_reg_y];
            }
            // 8XY3 - Bitwise XOR operation
            (8, _, _, 3) => {
                let v_reg_x = digit2 as usize;
                let v_reg_y = digit3 as usize;

                self.v_registers[v_reg_x] ^= self.v_registers[v_reg_y];
            }
            // 8XY4 - VX += VY ::: Store carry flag (1 or 0) in VF (flag) register after add
            (8, _, _, 4) => {
                let vx = digit2 as usize;
                let vy = digit3 as usize;
                
                let (new_vx, carry) = self.v_registers[vx].overflowing_add(self.v_registers[vy]);
                let new_vf = if carry { 1 } else { 0 };
                
                self.v_registers[vx] = new_vx;
                self.v_registers[0xF] = new_vf;
            }
            // 8XY5 - VX -= VY ::: Store carry(borrow) flag in VF register after subtraction
            (8, _, _, 5) => {
                let vx = digit2 as usize;
                let vy = digit3 as usize;
                
                let (new_vx, borrow) = self.v_registers[vx].overflowing_sub(self.v_registers[vy]);
                let new_vf = if borrow { 0 } else { 1 }; 

                self.v_registers[vx] = new_vx;
                self.v_registers[0xF] = new_vf;
            }
            // 8XY6 - VX >>= 1 ::: Right shift value in VX register, store dropped bit in VF
            (8, _, _, 6) => {
                let vx = digit2 as usize;
                let lsb = self.v_registers[vx] & 1;
                self.v_registers[vx] >>= 1;
                self.v_registers[0xF] = lsb;
            }
            (_, _, _, _) => unimplemented!("Unimplemented opcode: {}", &op),
            // 
            
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
