// cpu.rs

// TODO: in the instruction match statement, all of the register ones have `let result` inside the if statement
//       move this up to match all of the other ones (or move all of the other ones down, which would probably be better anyways)

use crate::Bus;

const DEBUG: bool = false;

#[derive(Copy, Clone)]
pub struct Flag {
    pub interrupt: bool,
    pub carry: bool,
    pub zero: bool,
}

impl std::convert::From<Flag> for u8  {
    fn from(flag: Flag) -> u8 {
        (if flag.interrupt { 1 } else { 0 }) << 2 |
        (if flag.carry     { 1 } else { 0 }) << 1 |
        (if flag.zero      { 1 } else { 0 }) << 0
    }
}

impl std::convert::From<u8> for Flag {
    fn from(byte: u8) -> Self {
        let interrupt = ((byte >> 2) & 1) != 0;
        let carry     = ((byte >> 1) & 1) != 0;
        let zero      = ((byte >> 0) & 1) != 0;
        Flag { interrupt, carry, zero }
    }
}

#[derive(Debug)]
pub enum Exception {
    DivideByZero,
    InvalidOpcode(u32),
}

#[derive(Debug)]
pub enum Interrupt {
    Exception(Exception),
    Request(u8), // u8 contains the interrupt vector value
}

pub struct Cpu {
    pub instruction_pointer: u32,
    pub stack_pointer: u32,

    pub register: [u32; 32],
    pub flag: Flag,
    pub halted: bool,

    pub bus: Bus,
}

impl Cpu {
    pub fn new(bus: Bus) -> Self {
        Cpu {
            instruction_pointer: 0xF0000000,
            stack_pointer: 0x00000000,
            register: [0; 32],
            flag: Flag { interrupt: false, carry: false, zero: false },
            halted: false,
            bus,
        }
    }
    fn check_condition(&self, condition: Condition) -> bool {
        match condition {
            Condition::Always => true,
            Condition::Zero => self.flag.zero,
            Condition::NotZero => !self.flag.zero,
            Condition::Carry => self.flag.carry,
            Condition::NotCarry => !self.flag.carry,
            Condition::GreaterThan => !self.flag.carry && !self.flag.zero,
            Condition::LessThanEqualTo => self.flag.carry || self.flag.zero,
        }
    }
    fn relative_to_absolute(&self, relative_address: u32) -> u32 {
        self.instruction_pointer.wrapping_add(relative_address)
    }
    fn read_source(&self, source: Operand) -> (u32, u32) {
        let mut instruction_pointer_offset = 2; // increment past opcode half
        let source_value = match source {
            Operand::Register => {
                let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                let value = self.read_register(register);
                instruction_pointer_offset += 1; // increment past 8 bit register number
                value
            }
            Operand::RegisterPtr => {
                let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                let pointer = self.read_register(register);
                let value = self.bus.memory.read_32(pointer);
                instruction_pointer_offset += 1; // increment past 8 bit register number
                value
            }
            Operand::Immediate8 => {
                let value = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                instruction_pointer_offset += 1; // increment past 8 bit immediate
                value as u32
            }
            Operand::Immediate16 => {
                let value = self.bus.memory.read_16(self.instruction_pointer + instruction_pointer_offset);
                instruction_pointer_offset += 2; // increment past 16 bit immediate
                value as u32
            }
            Operand::Immediate32 => {
                let value = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                instruction_pointer_offset += 4; // increment past 32 bit immediate
                value
            }
            Operand::ImmediatePtr(size) => {
                let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                let value = match size {
                    Size::Byte => self.bus.memory.read_8(pointer) as u32,
                    Size::Half => self.bus.memory.read_16(pointer) as u32,
                    Size::Word => self.bus.memory.read_32(pointer),
                };
                instruction_pointer_offset += 4; // increment past 32 bit pointer
                value
            }
        };
        (source_value, instruction_pointer_offset)
    }
    pub fn read_register(&self, register: u8) -> u32 {
        match register {
            0..=31 => self.register[register as usize],
            32 => self.stack_pointer,
            _ => panic!("Invalid register: {}", register),
        }
    }
    pub fn write_register(&mut self, register: u8, word: u32) {
        match register {
            0..=31 => self.register[register as usize] = word,
            32 => self.stack_pointer = word,
            _ => panic!("Invalid register: {}", register),
        };
    }
    pub fn print_registers(&mut self) {
        for index in 0..2 {
            println!("r{}: {:#010X} | r{}:  {:#010X} | r{}: {:#010X} | r{}: {:#010X}",
                index, self.register[index],
                index + 8, self.register[index + 8],
                index + 16, self.register[index + 16],
                index + 24, self.register[index + 24]
            );
        }
        for index in 2..8 {
            println!("r{}: {:#010X} | r{}: {:#010X} | r{}: {:#010X} | r{}: {:#010X}",
                index, self.register[index],
                index + 8, self.register[index + 8],
                index + 16, self.register[index + 16],
                index + 24, self.register[index + 24]
            );
        }
        println!("rsp: {:#010X}", self.stack_pointer);
    }
    pub fn push_stack_8(&mut self, byte: u8) {
        let decremented_stack_pointer = self.stack_pointer.overflowing_sub(1);
        self.stack_pointer = decremented_stack_pointer.0;
        if decremented_stack_pointer.1 {
            // TODO: stack overflow exception
        }
        self.bus.memory.write_8(self.stack_pointer, byte);
    }
    pub fn pop_stack_8(&mut self) -> u8 {
        let byte = self.bus.memory.read_8(self.stack_pointer);
        let incremented_stack_pointer = self.stack_pointer.overflowing_add(1);
        self.stack_pointer = incremented_stack_pointer.0;
        if incremented_stack_pointer.1 {
            // TODO: stack overflow exception
        }
        byte
    }
    pub fn push_stack_16(&mut self, half: u16) {
        let decremented_stack_pointer = self.stack_pointer.overflowing_sub(2);
        self.stack_pointer = decremented_stack_pointer.0;
        if decremented_stack_pointer.1 {
            // TODO: stack overflow exception
        }
        self.bus.memory.write_16(self.stack_pointer, half);
    }
    pub fn pop_stack_16(&mut self) -> u16 {
        let half = self.bus.memory.read_16(self.stack_pointer);
        let incremented_stack_pointer = self.stack_pointer.overflowing_add(2);
        self.stack_pointer = incremented_stack_pointer.0;
        if incremented_stack_pointer.1 {
            // TODO: stack overflow exception
        }
        half
    }
    pub fn push_stack_32(&mut self, word: u32) {
        let decremented_stack_pointer = self.stack_pointer.overflowing_sub(4);
        self.stack_pointer = decremented_stack_pointer.0;
        if decremented_stack_pointer.1 {
            // TODO: stack overflow exception
        }
        self.bus.memory.write_32(self.stack_pointer, word);
    }
    pub fn pop_stack_32(&mut self) -> u32 {
        let word = self.bus.memory.read_32(self.stack_pointer);
        let incremented_stack_pointer = self.stack_pointer.overflowing_add(4);
        self.stack_pointer = incremented_stack_pointer.0;
        if incremented_stack_pointer.1 {
            // TODO: stack overflow exception
        }
        word
    }
    pub fn interrupt(&mut self, interrupt: Interrupt) {
        if DEBUG { println!("interrupt(): enabled: {}", self.flag.interrupt); }
        if self.flag.interrupt {
            match interrupt {
                Interrupt::Request(vector) => {
                    self.handle_interrupt(vector as u16);
                }
                Interrupt::Exception(exception) => {
                    match exception {
                        Exception::DivideByZero => {
                            let vector: u16 = 0;
                            self.handle_exception(vector, None);
                        }
                        Exception::InvalidOpcode(opcode) => {
                            let vector: u16 = 1;
                            self.handle_exception(vector, Some(opcode));
                        }
                    }
                }
            }
        }
    }
    fn handle_interrupt(&mut self, vector: u16) {
        if DEBUG { println!("interrupt!!! vector: {:#04X}", vector); }
        let address_of_pointer = vector as u32 * 4;
        let address = self.bus.memory.read_32(address_of_pointer);
        self.push_stack_32(self.instruction_pointer);
        self.push_stack_8(u8::from(self.flag));
        self.flag.interrupt = false; // prevent interrupts while already servicing an interrupt
        self.instruction_pointer = address;
    }
    fn handle_exception(&mut self, vector: u16, operand: Option<u32>) {
        if DEBUG { println!("exception!!! vector: {:#04X}, operand: {:?}", vector, operand); }
        let address_of_pointer = (256 + vector) as u32 * 4;
        let address = self.bus.memory.read_32(address_of_pointer);
        self.push_stack_32(self.instruction_pointer);
        self.push_stack_8(u8::from(self.flag));
        if let Some(operand) = operand {
            self.push_stack_32(operand);
        }
        self.flag.interrupt = false; // prevent interrupts while already servicing an interrupt
        self.instruction_pointer = address;
    }
    // execute instruction from memory at the current instruction pointer
    pub fn execute_memory_instruction(&mut self) {
        let opcode = self.bus.memory.read_16(self.instruction_pointer);

        if let Some(instruction) = Instruction::from_half(opcode) {
            if DEBUG { println!("{:#010X}: {:?}", self.instruction_pointer, instruction); }
            self.instruction_pointer = self.execute_instruction(instruction);
        } else {
            let size = ((opcode & 0b1100000000000000) >> 14) as u8;
            let instruction = ((opcode & 0b0011111100000000) >> 8) as u8;
            let empty = ((opcode & 0b0000000010000000) >> 7) as u8;
            let condition = ((opcode & 0b0000000001110000) >> 4) as u8;
            let destination = ((opcode & 0b0000000000001100) >> 2) as u8;
            let source = (opcode & 0b0000000000000011) as u8;

            println!("{:#010X}: bad opcode {:#06X}", self.instruction_pointer, opcode);
            println!("size instr  . cond dest src");
            println!("{:02b}   {:06b} {:01b} {:03b}  {:02b}   {:02b}", size, instruction, empty, condition, destination, source);
            println!("dumping RAM");
            self.bus.memory.dump();
            panic!("bad opcode");
        }
    }
    // execute one instruction and return the next instruction pointer value
    fn execute_instruction(&mut self, instruction: Instruction) -> u32 {
        match instruction {
            Instruction::Nop() => {
                self.instruction_pointer + 2 // increment past opcode half
            }
            Instruction::Halt(condition) => {
                let instruction_pointer_offset = 2; // increment past opcode half
                let should_run = self.check_condition(condition);
                if should_run {
                    self.halted = true;
                    //if DEBUG { self.print_registers(); }
                }
                self.instruction_pointer + instruction_pointer_offset
            }
            Instruction::Brk(condition) => {
                let instruction_pointer_offset = 2; // increment past opcode half
                let should_run = self.check_condition(condition);
                if should_run {
                    //self.breakpoint = true;
                    println!("Breakpoint reached");
                    self.print_registers();
                }
                self.instruction_pointer + instruction_pointer_offset
            }

            Instruction::Add(size, condition, destination, source) => {
                let (source_value, mut instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                match destination {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let result = (self.read_register(register) as u8).overflowing_add(source_value as u8);
                                    self.write_register(register, (self.read_register(register) & 0xFFFFFF00) | (result.0 as u32));
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let result = (self.read_register(register) as u16).overflowing_add(source_value as u16);
                                    self.write_register(register, (self.read_register(register) & 0xFFFF0000) | (result.0 as u32));
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let result = self.read_register(register).overflowing_add(source_value);
                                    self.write_register(register, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer).overflowing_add(source_value as u8);
                                if should_run {
                                    self.bus.memory.write_8(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer).overflowing_add(source_value as u16);
                                if should_run {
                                    self.bus.memory.write_16(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer).overflowing_add(source_value);
                                if should_run {
                                    self.bus.memory.write_32(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer).overflowing_add(source_value as u8);
                                if should_run {
                                    self.bus.memory.write_8(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer).overflowing_add(source_value as u16);
                                if should_run {
                                    self.bus.memory.write_16(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer).overflowing_add(source_value);
                                if should_run {
                                    self.bus.memory.write_32(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                        }
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }
            Instruction::Inc(size, condition, source) => {
                let mut instruction_pointer_offset = 2; // increment past opcode half
                let should_run = self.check_condition(condition);
                match source {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let result = (self.read_register(register) as u8).overflowing_add(1);
                                    self.write_register(register, (self.read_register(register) & 0xFFFFFF00) | (result.0 as u32));
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let result = (self.read_register(register) as u16).overflowing_add(1);
                                    self.write_register(register, (self.read_register(register) & 0xFFFF0000) | (result.0 as u32));
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let result = self.read_register(register).overflowing_add(1);
                                    self.write_register(register, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let result = self.bus.memory.read_8(pointer).overflowing_add(1);
                                    self.bus.memory.write_8(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let result = self.bus.memory.read_16(pointer).overflowing_add(1);
                                    self.bus.memory.write_16(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let result = self.bus.memory.read_32(pointer).overflowing_add(1);
                                    self.bus.memory.write_32(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let result = self.bus.memory.read_8(pointer).overflowing_add(1);
                                    self.bus.memory.write_8(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let result = self.bus.memory.read_16(pointer).overflowing_add(1);
                                    self.bus.memory.write_16(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let result = self.bus.memory.read_32(pointer).overflowing_add(1);
                                    self.bus.memory.write_32(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                        }
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }
            Instruction::Sub(size, condition, destination, source) => {
                let (source_value, mut instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                match destination {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let result = (self.read_register(register) as u8).overflowing_sub(source_value as u8);
                                    self.write_register(register, (self.read_register(register) & 0xFFFFFF00) | (result.0 as u32));
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let result = (self.read_register(register) as u16).overflowing_sub(source_value as u16);
                                    self.write_register(register, (self.read_register(register) & 0xFFFF0000) | (result.0 as u32));
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let result = self.read_register(register).overflowing_sub(source_value);
                                    self.write_register(register, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer).overflowing_sub(source_value as u8);
                                if should_run {
                                    self.bus.memory.write_8(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer).overflowing_sub(source_value as u16);
                                if should_run {
                                    self.bus.memory.write_16(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer).overflowing_sub(source_value);
                                if should_run {
                                    self.bus.memory.write_32(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer).overflowing_sub(source_value as u8);
                                if should_run {
                                    self.bus.memory.write_8(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer).overflowing_sub(source_value as u16);
                                if should_run {
                                    self.bus.memory.write_16(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer).overflowing_sub(source_value);
                                if should_run {
                                    self.bus.memory.write_32(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                        }
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }
            Instruction::Dec(size, condition, source) => {
                let mut instruction_pointer_offset = 2; // increment past opcode half
                let should_run = self.check_condition(condition);
                match source {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let result = (self.read_register(register) as u8).overflowing_sub(1);
                                    self.write_register(register, (self.read_register(register) & 0xFFFFFF00) | (result.0 as u32));
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let result = (self.read_register(register) as u16).overflowing_sub(1);
                                    self.write_register(register, (self.read_register(register) & 0xFFFF0000) | (result.0 as u32));
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let result = self.read_register(register).overflowing_sub(1);
                                    self.write_register(register, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let result = self.bus.memory.read_8(pointer).overflowing_sub(1);
                                    self.bus.memory.write_8(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let result = self.bus.memory.read_16(pointer).overflowing_sub(1);
                                    self.bus.memory.write_16(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let result = self.bus.memory.read_32(pointer).overflowing_sub(1);
                                    self.bus.memory.write_32(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let result = self.bus.memory.read_8(pointer).overflowing_sub(1);
                                    self.bus.memory.write_8(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let result = self.bus.memory.read_16(pointer).overflowing_sub(1);
                                    self.bus.memory.write_16(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let result = self.bus.memory.read_32(pointer).overflowing_sub(1);
                                    self.bus.memory.write_32(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                        }
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }
            Instruction::Mul(size, condition, destination, source) => {
                let (source_value, mut instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                match destination {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let result = (self.read_register(register) as u8).overflowing_mul(source_value as u8);
                                    self.write_register(register, (self.read_register(register) & 0xFFFFFF00) | (result.0 as u32));
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let result = (self.read_register(register) as u16).overflowing_mul(source_value as u16);
                                    self.write_register(register, (self.read_register(register) & 0xFFFF0000) | (result.0 as u32));
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let result = self.read_register(register).overflowing_mul(source_value);
                                    self.write_register(register, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer).overflowing_mul(source_value as u8);
                                if should_run {
                                    self.bus.memory.write_8(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer).overflowing_mul(source_value as u16);
                                if should_run {
                                    self.bus.memory.write_16(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer).overflowing_mul(source_value);
                                if should_run {
                                    self.bus.memory.write_32(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer).overflowing_mul(source_value as u8);
                                if should_run {
                                    self.bus.memory.write_8(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer).overflowing_mul(source_value as u16);
                                if should_run {
                                    self.bus.memory.write_16(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer).overflowing_mul(source_value);
                                if should_run {
                                    self.bus.memory.write_32(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                        }
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }
            Instruction::Div(size, condition, destination, source) => {
                let (source_value, mut instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                match destination {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let result = (self.read_register(register) as u8).overflowing_div(source_value as u8);
                                    self.write_register(register, (self.read_register(register) & 0xFFFFFF00) | (result.0 as u32));
                                    self.flag.zero = result.0 == 0;
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let result = (self.read_register(register) as u16).overflowing_div(source_value as u16);
                                    self.write_register(register, (self.read_register(register) & 0xFFFF0000) | (result.0 as u32));
                                    self.flag.zero = result.0 == 0;
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let result = self.read_register(register).overflowing_div(source_value);
                                    self.write_register(register, result.0);
                                    self.flag.zero = result.0 == 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer).overflowing_div(source_value as u8);
                                if should_run {
                                    self.bus.memory.write_8(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer).overflowing_div(source_value as u16);
                                if should_run {
                                    self.bus.memory.write_16(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer).overflowing_div(source_value);
                                if should_run {
                                    self.bus.memory.write_32(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer).overflowing_div(source_value as u8);
                                if should_run {
                                    self.bus.memory.write_8(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer).overflowing_div(source_value as u16);
                                if should_run {
                                    self.bus.memory.write_16(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer).overflowing_div(source_value);
                                if should_run {
                                    self.bus.memory.write_32(pointer, result.0);
                                    self.flag.zero = result.0 == 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }
            Instruction::Rem(size, condition, destination, source) => {
                let (source_value, mut instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                match destination {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let result = (self.read_register(register) as u8) % source_value as u8;
                                    self.write_register(register, (self.read_register(register) & 0xFFFFFF00) | (result as u32));
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let result = (self.read_register(register) as u16) % source_value as u16;
                                    self.write_register(register, (self.read_register(register) & 0xFFFF0000) | (result as u32));
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let result = self.read_register(register) % source_value;
                                    self.write_register(register, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer) % source_value as u8;
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer) % source_value as u16;
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer) % source_value;
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer) % source_value as u8;
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer) % source_value as u16;
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer) % source_value;
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }

            Instruction::And(size, condition, destination, source) => {
                let (source_value, mut instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                match destination {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let result = (self.read_register(register) as u8) & source_value as u8;
                                    self.write_register(register, (self.read_register(register) & 0xFFFFFF00) | (result as u32));
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let result = (self.read_register(register) as u16) & source_value as u16;
                                    self.write_register(register, (self.read_register(register) & 0xFFFF0000) | (result as u32));
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let result = self.read_register(register) & source_value;
                                    self.write_register(register, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer) & source_value as u8;
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer) & source_value as u16;
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer) & source_value;
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer) & source_value as u8;
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer) & source_value as u16;
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer) & source_value;
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }
            Instruction::Or(size, condition, destination, source) => {
                let (source_value, mut instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                match destination {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let result = (self.read_register(register) as u8) | source_value as u8;
                                    self.write_register(register, (self.read_register(register) & 0xFFFFFF00) | (result as u32));
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let result = (self.read_register(register) as u16) | source_value as u16;
                                    self.write_register(register, (self.read_register(register) & 0xFFFF0000) | (result as u32));
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let result = self.read_register(register) | source_value;
                                    self.write_register(register, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer) | source_value as u8;
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer) | source_value as u16;
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer) | source_value;
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer) | source_value as u8;
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer) | source_value as u16;
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer) | source_value;
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }
            Instruction::Xor(size, condition, destination, source) => {
                let (source_value, mut instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                match destination {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let result = (self.read_register(register) as u8) ^ source_value as u8;
                                    self.write_register(register, (self.read_register(register) & 0xFFFFFF00) | (result as u32));
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let result = (self.read_register(register) as u16) ^ source_value as u16;
                                    self.write_register(register, (self.read_register(register) & 0xFFFF0000) | (result as u32));
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let result = self.read_register(register) ^ source_value;
                                    self.write_register(register, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer) ^ source_value as u8;
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer) ^ source_value as u16;
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer) ^ source_value;
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer) ^ source_value as u8;
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer) ^ source_value as u16;
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer) ^ source_value;
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }
            Instruction::Not(size, condition, source) => {
                let mut instruction_pointer_offset = 2; // increment past opcode half
                let should_run = self.check_condition(condition);
                match source {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let result = !self.read_register(register) as u8;
                                    self.write_register(register, (self.read_register(register) & 0xFFFFFF00) | (result as u32));
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let result = !self.read_register(register) as u16;
                                    self.write_register(register, (self.read_register(register) & 0xFFFF0000) | (result as u32));
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let result = !self.read_register(register);
                                    self.write_register(register, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        match size {
                            Size::Byte => {
                                let result =!self.bus.memory.read_8(pointer);
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Half => {
                                let result = !self.bus.memory.read_16(pointer);
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Word => {
                                let result = !self.bus.memory.read_32(pointer);
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                let result = !self.bus.memory.read_8(pointer);
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Half => {
                                let result = !self.bus.memory.read_16(pointer);
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                            Size::Word => {
                                let result = !self.bus.memory.read_32(pointer);
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                    self.flag.zero = result == 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }
            Instruction::Sla(size, condition, destination, source) => {
                let (source_value, mut instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                match destination {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let result = (self.read_register(register) as u8) << source_value;
                                    self.write_register(register, (self.read_register(register) & 0xFFFFFF00) | (result as u32));
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 7) != 0;
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let result = (self.read_register(register) as u16) << source_value;
                                    self.write_register(register, (self.read_register(register) & 0xFFFF0000) | (result as u32));
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 15) != 0;
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let result = self.read_register(register) << source_value;
                                    self.write_register(register, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 31) != 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer) << source_value;
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 7) != 0;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer) << source_value;
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 15) != 0;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer) << source_value;
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 31) != 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer) << source_value;
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 7) != 0;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer) << source_value;
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 15) != 0;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer) << source_value;
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 31) != 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }
            Instruction::Sra(size, condition, destination, source) => {
                let (source_value, mut instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                match destination {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let result = (self.read_register(register) as u8) >> source_value as i32;
                                    self.write_register(register, (self.read_register(register) & 0xFFFFFF00) | (result as u32));
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let result = (self.read_register(register) as u16) >> source_value as i32;
                                    self.write_register(register, (self.read_register(register) & 0xFFFF0000) | (result as u32));
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let result = self.read_register(register) >> source_value as i32;
                                    self.write_register(register, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer) >> source_value as i32;
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer) >> source_value as i32;
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer) >> source_value as i32;
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer) >> source_value as i32;
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer) >> source_value as i32;
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer) >> source_value as i32;
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }
            Instruction::Srl(size, condition, destination, source) => {
                let (source_value, mut instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                match destination {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let result = (self.read_register(register) as u8) >> source_value;
                                    self.write_register(register, (self.read_register(register) & 0xFFFFFF00) | (result as u32));
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let result = (self.read_register(register) as u16) >> source_value;
                                    self.write_register(register, (self.read_register(register) & 0xFFFF0000) | (result as u32));
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let result = self.read_register(register) >> source_value;
                                    self.write_register(register, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer) >> source_value;
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer) >> source_value;
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer) >> source_value;
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer) >> source_value;
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer) >> source_value;
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer) >> source_value;
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }
            Instruction::Rol(size, condition, destination, source) => {
                let (source_value, mut instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                match destination {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let result = (self.read_register(register) as u8).rotate_left(source_value);
                                    self.write_register(register, (self.read_register(register) & 0xFFFFFF00) | (result as u32));
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 7) != 0;
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let result = (self.read_register(register) as u16).rotate_left(source_value);
                                    self.write_register(register, (self.read_register(register) & 0xFFFF0000) | (result as u32));
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 15) != 0;
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let result = self.read_register(register).rotate_left(source_value);
                                    self.write_register(register, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 31) != 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer).rotate_left(source_value);
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 7) != 0;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer).rotate_left(source_value);
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 15) != 0;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer).rotate_left(source_value);
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 31) != 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer).rotate_left(source_value);
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 7) != 0;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer).rotate_left(source_value);
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 15) != 0;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer).rotate_left(source_value);
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 31) != 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }
            Instruction::Ror(size, condition, destination, source) => {
                let (source_value, mut instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                match destination {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let result = (self.read_register(register) as u8).rotate_right(source_value);
                                    self.write_register(register, (self.read_register(register) & 0xFFFFFF00) | (result as u32));
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let result = (self.read_register(register) as u16).rotate_right(source_value);
                                    self.write_register(register, (self.read_register(register) & 0xFFFF0000) | (result as u32));
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let result = self.read_register(register).rotate_right(source_value);
                                    self.write_register(register, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer).rotate_right(source_value);
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer).rotate_right(source_value);
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer).rotate_right(source_value);
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer).rotate_right(source_value);
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer).rotate_right(source_value);
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer).rotate_right(source_value);
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                    self.flag.zero = result == 0;
                                    self.flag.carry = source_value & (1 << 0) != 0;
                                }
                            }
                        }
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }

            Instruction::Bse(size, condition, destination, source) => {
                let (source_value, mut instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                match destination {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                let result = self.read_register(register) as u8 | (1 << source_value);
                                if should_run {
                                    self.write_register(register, (self.read_register(register) & 0xFFFFFF00) | (result as u32));
                                }
                            }
                            Size::Half => {
                                let result = self.read_register(register) as u16 | (1 << source_value);
                                if should_run {
                                    self.write_register(register, (self.read_register(register) & 0xFFFF0000) | (result as u32));
                                }
                            }
                            Size::Word => {
                                let result = self.read_register(register) | (1 << source_value);
                                if should_run {
                                    self.write_register(register, result);
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer) | (1 << source_value);
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer) | (1 << source_value);
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer) | (1 << source_value);
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer) | (1 << source_value);
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer) | (1 << source_value);
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer) | (1 << source_value);
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                }
                            }
                        }
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }
            Instruction::Bcl(size, condition, destination, source) => {
                let (source_value, mut instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                match destination {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                let result = self.read_register(register) as u8 & !(1 << source_value);
                                if should_run {
                                    self.write_register(register, (self.read_register(register) & 0xFFFFFF00) | (result as u32));
                                }
                            }
                            Size::Half => {
                                let result = self.read_register(register) as u16 & !(1 << source_value);
                                if should_run {
                                    self.write_register(register, (self.read_register(register) & 0xFFFF0000) | (result as u32));
                                }
                            }
                            Size::Word => {
                                let result = self.read_register(register) & !(1 << source_value);
                                if should_run {
                                    self.write_register(register, result);
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer) & !(1 << source_value);
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer) & !(1 << source_value);
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer) & !(1 << source_value);
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer) & !(1 << source_value);
                                if should_run {
                                    self.bus.memory.write_8(pointer, result);
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer) & !(1 << source_value);
                                if should_run {
                                    self.bus.memory.write_16(pointer, result);
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer) & !(1 << source_value);
                                if should_run {
                                    self.bus.memory.write_32(pointer, result);
                                }
                            }
                        }
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }
            Instruction::Bts(size, condition, destination, source) => {
                let (source_value, mut instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                match destination {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                let result = self.read_register(register) as u8 & (1 << source_value) == 0;
                                if should_run {
                                    self.flag.zero = result;
                                }
                            }
                            Size::Half => {
                                let result = self.read_register(register) as u16 & (1 << source_value) == 0;
                                if should_run {
                                    self.flag.zero = result;
                                }
                            }
                            Size::Word => {
                                let result = self.read_register(register) & (1 << source_value) == 0;
                                if should_run {
                                    self.flag.zero = result;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer) & (1 << source_value) == 0;
                                if should_run {
                                    self.flag.zero = result;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer) & (1 << source_value) == 0;
                                if should_run {
                                    self.flag.zero = result;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer) & (1 << source_value) == 0;
                                if should_run {
                                    self.flag.zero = result;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer) & (1 << source_value) == 0;
                                if should_run {
                                    self.flag.zero = result;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer) & (1 << source_value) == 0;
                                if should_run {
                                    self.flag.zero = result;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer) & (1 << source_value) == 0;
                                if should_run {
                                    self.flag.zero = result;
                                }
                            }
                        }
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }

            Instruction::Cmp(size, condition, destination, source) => {
                let (source_value, mut instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                match destination {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let result = (self.read_register(register) as u8).overflowing_sub(source_value as u8);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let result = (self.read_register(register) as u16).overflowing_sub(source_value as u16);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let result = self.read_register(register).overflowing_sub(source_value);
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer).overflowing_sub(source_value as u8);
                                if should_run {
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer).overflowing_sub(source_value as u16);
                                if should_run {
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer).overflowing_sub(source_value);
                                if should_run {
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                let result = self.bus.memory.read_8(pointer).overflowing_sub(source_value as u8);
                                if should_run {
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Half => {
                                let result = self.bus.memory.read_16(pointer).overflowing_sub(source_value as u16);
                                if should_run {
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                            Size::Word => {
                                let result = self.bus.memory.read_32(pointer).overflowing_sub(source_value);
                                if should_run {
                                    self.flag.zero = result.0 == 0;
                                    self.flag.carry = result.1;
                                }
                            }
                        }
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }
            Instruction::Mov(size, condition, destination, source) => {
                let (source_value, mut instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                match destination {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    self.write_register(register, (self.read_register(register) & 0xFFFFFF00) | (source_value & 0x000000FF));
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    self.write_register(register, (self.read_register(register) & 0xFFFF0000) | (source_value & 0x0000FFFF));
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    self.write_register(register, source_value);
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    self.bus.memory.write_8(pointer, source_value as u8);
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    self.bus.memory.write_16(pointer, source_value as u16);
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    self.bus.memory.write_32(pointer, source_value);
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    self.bus.memory.write_8(pointer, source_value as u8);
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    self.bus.memory.write_16(pointer, source_value as u16);
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    self.bus.memory.write_32(pointer, source_value);
                                }
                            }
                        }
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }
            Instruction::Movz(size, condition, destination, source) => {
                let (source_value, mut instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                match destination {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    self.write_register(register, source_value & 0x000000FF);
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    self.write_register(register, source_value & 0x0000FFFF);
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    self.write_register(register, source_value);
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    _ => panic!("MOVZ only operates on registers"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }

            Instruction::Jmp(condition, source) => {
                let (source_value, instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                if should_run {
                    source_value
                } else {
                    self.instruction_pointer + instruction_pointer_offset
                }
            }
            Instruction::Call(condition, source) => {
                let (source_value, instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                if should_run {
                    self.push_stack_32(self.instruction_pointer + instruction_pointer_offset);
                    source_value
                } else {
                    self.instruction_pointer + instruction_pointer_offset
                }
            }
            Instruction::Loop(condition, source) => {
                let (source_value, instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                let result = self.read_register(31).overflowing_sub(1);
                self.write_register(31, result.0);
                if should_run {
                    if result.0 != 0 {
                        source_value
                    } else {
                        self.instruction_pointer + instruction_pointer_offset
                    }
                } else {
                    self.instruction_pointer + instruction_pointer_offset
                }
            }
            Instruction::Rjmp(condition, source) => {
                let (source_value, instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                if should_run {
                    self.relative_to_absolute(source_value)
                } else {
                    self.instruction_pointer + instruction_pointer_offset
                }
            }
            Instruction::Rcall(condition, source) => {
                let (source_value, instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                if should_run {
                    self.push_stack_32(self.instruction_pointer + instruction_pointer_offset);
                    self.relative_to_absolute(source_value)
                } else {
                    self.instruction_pointer + instruction_pointer_offset
                }
            }
            Instruction::Rloop(condition, source) => {
                let (source_value, instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                let result = self.read_register(31).overflowing_sub(1);
                self.write_register(31, result.0);
                if should_run {
                    if result.0 != 0 {
                        self.relative_to_absolute(source_value)
                    } else {
                        self.instruction_pointer + instruction_pointer_offset
                    }
                } else {
                    self.instruction_pointer + instruction_pointer_offset
                }
            }

            Instruction::Rta(condition, destination, source) => {
                let (source_value, mut instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                match destination {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        if should_run {
                            self.write_register(register, self.relative_to_absolute(source_value));
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.relative_to_absolute(self.read_register(register));
                        if should_run {
                            // INFO: register contains a relative address instead of an absolute address
                            self.bus.memory.write_32(pointer, self.relative_to_absolute(source_value));
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.relative_to_absolute(self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset));
                        if should_run {
                            self.bus.memory.write_32(pointer, self.relative_to_absolute(source_value));
                        }
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }

            Instruction::Push(size, condition, source) => {
                let (source_value, instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                match size {
                    Size::Byte => {
                        if should_run {
                            self.push_stack_8(source_value as u8);
                        }
                    }
                    Size::Half => {
                        if should_run {
                            self.push_stack_16(source_value as u16);
                        }
                    }
                    Size::Word => {
                        if should_run {
                            self.push_stack_32(source_value);
                        }
                    }
                }
                self.instruction_pointer + instruction_pointer_offset
            }
            Instruction::Pop(size, condition, source) => {
                let mut instruction_pointer_offset = 2; // increment past opcode half
                let should_run = self.check_condition(condition);
                match source {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let value = self.pop_stack_8() as u32;
                                    self.write_register(register, (self.read_register(register) & 0xFFFFFF00) | value);
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let value = self.pop_stack_16() as u32;
                                    self.write_register(register, (self.read_register(register) & 0xFFFF0000) | value);
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let value = self.pop_stack_32();
                                    self.write_register(register, value);
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let value = self.pop_stack_8();
                                    self.bus.memory.write_8(pointer, value);
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let value = self.pop_stack_16();
                                    self.bus.memory.write_16(pointer, value);
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let value = self.pop_stack_32();
                                    self.bus.memory.write_32(pointer, value);
                                }
                            }
                        }
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        match size {
                            Size::Byte => {
                                if should_run {
                                    let value = self.pop_stack_8();
                                    self.bus.memory.write_8(pointer, value);
                                }
                            }
                            Size::Half => {
                                if should_run {
                                    let value = self.pop_stack_16();
                                    self.bus.memory.write_16(pointer, value);
                                }
                            }
                            Size::Word => {
                                if should_run {
                                    let value = self.pop_stack_32();
                                    self.bus.memory.write_32(pointer, value);
                                }
                            }
                        }
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }
            Instruction::Ret(condition) => {
                let instruction_pointer_offset = 2; // increment past opcode half
                let should_run = self.check_condition(condition);
                if should_run {
                    self.pop_stack_32()
                } else {
                    self.instruction_pointer + instruction_pointer_offset
                }
            }
            Instruction::Reti(condition) => {
                let instruction_pointer_offset = 2; // increment past opcode half
                let should_run = self.check_condition(condition);
                if should_run {
                    self.flag = Flag::from(self.pop_stack_8());
                    self.pop_stack_32()
                } else {
                    self.instruction_pointer + instruction_pointer_offset
                }
            }

            Instruction::In(condition, destination, source) => {
                let (source_value, mut instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                match destination {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let value = self.bus.read_io(source_value);
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                        if should_run {
                            self.write_register(register, value);
                        }
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        let value = self.bus.read_io(source_value);
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                        if should_run {
                            self.bus.memory.write_32(pointer, value);
                        }
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        let value = self.bus.read_io(source_value);
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                        if should_run {
                            self.bus.memory.write_32(pointer, value);
                        }
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }
            Instruction::Out(condition, destination, source) => {
                let (source_value, mut instruction_pointer_offset) = self.read_source(source);
                let should_run = self.check_condition(condition);
                match destination {
                    Operand::Register => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                        if should_run {
                            self.bus.write_io(self.read_register(register), source_value);
                        }
                    }
                    Operand::RegisterPtr => {
                        let register = self.bus.memory.read_8(self.instruction_pointer + instruction_pointer_offset);
                        let pointer = self.read_register(register);
                        instruction_pointer_offset += 1; // increment past 8 bit register number
                        if should_run {
                            self.bus.write_io(self.bus.memory.read_32(pointer), source_value);
                        }
                    }
                    Operand::ImmediatePtr(_) => {
                        let pointer = self.bus.memory.read_32(self.instruction_pointer + instruction_pointer_offset);
                        instruction_pointer_offset += 4; // increment past 32 bit pointer
                        if should_run {
                            self.bus.write_io(self.bus.memory.read_32(pointer), source_value);
                        }
                    }
                    _ => panic!("Attempting to use an immediate value as a destination"),
                }
                self.instruction_pointer + instruction_pointer_offset
            }

            Instruction::Ise(condition) => {
                let instruction_pointer_offset = 2; // increment past opcode half
                let should_run = self.check_condition(condition);
                if should_run {
                    self.flag.interrupt = true;
                }
                self.instruction_pointer + instruction_pointer_offset
            }
            Instruction::Icl(condition) => {
                let instruction_pointer_offset = 2; // increment past opcode half
                let should_run = self.check_condition(condition);
                if should_run {
                    self.flag.interrupt = false;
                }
                self.instruction_pointer + instruction_pointer_offset
            }
        }
    }
}

#[derive(Debug)]
enum Operand {
    Register,
    RegisterPtr,
    Immediate8,
    Immediate16,
    Immediate32,
    ImmediatePtr(Size),
}

#[derive(Copy, Clone, Debug)]
enum Size {
    Byte,
    Half,
    Word,
}

#[derive(Debug)]
enum Condition {
    Always,
    Zero,
    NotZero,
    Carry,
    NotCarry,
    GreaterThan,
    // GreaterThanEqualTo is equivalent to NotCarry
    // LessThan is equivalent to Carry
    LessThanEqualTo,
}

#[derive(Debug)]
enum Instruction {
    Nop(),
    Halt(Condition),
    Brk(Condition),

    Add(Size, Condition, Operand, Operand),
    Inc(Size, Condition, Operand),
    Sub(Size, Condition, Operand, Operand),
    Dec(Size, Condition, Operand),

    Mul(Size, Condition, Operand, Operand),
    Div(Size, Condition, Operand, Operand),
    Rem(Size, Condition, Operand, Operand),

    And(Size, Condition, Operand, Operand),
    Or(Size, Condition, Operand, Operand),
    Xor(Size, Condition, Operand, Operand),
    Not(Size, Condition, Operand),

    Sla(Size, Condition, Operand, Operand),
    Rol(Size, Condition, Operand, Operand),

    Sra(Size, Condition, Operand, Operand),
    Srl(Size, Condition, Operand, Operand),
    Ror(Size, Condition, Operand, Operand),

    Bse(Size, Condition, Operand, Operand),
    Bcl(Size, Condition, Operand, Operand),
    Bts(Size, Condition, Operand, Operand),

    Cmp(Size, Condition, Operand, Operand),
    Mov(Size, Condition, Operand, Operand),
    Movz(Size, Condition, Operand, Operand),

    Jmp(Condition, Operand),
    Call(Condition, Operand),
    Loop(Condition, Operand),

    Rjmp(Condition, Operand),
    Rcall(Condition, Operand),
    Rloop(Condition, Operand),

    Rta(Condition, Operand, Operand),

    Push(Size, Condition, Operand),
    Pop(Size, Condition, Operand),
    Ret(Condition),
    Reti(Condition),

    In(Condition, Operand, Operand),
    Out(Condition, Operand, Operand),

    Ise(Condition),
    Icl(Condition),
}

impl Instruction {
    fn from_half(half: u16) -> Option<Instruction> {
        // see encoding.md for more info
        let size = match ((half >> 14) as u8) & 0b00000011 {
            0x00 => Size::Byte,
            0x01 => Size::Half,
            0x02 => Size::Word,
            _ => return None,
        };
        let opcode = ((half >> 8) as u8) & 0b00111111;
        let source = match ((half & 0x000F) as u8) & 0b00000011 {
            0x00 => Operand::Register,
            0x01 => Operand::RegisterPtr,
            0x02 => match size {
                Size::Byte => Operand::Immediate8,
                Size::Half => Operand::Immediate16,
                Size::Word => Operand::Immediate32,
            },
            0x03 => Operand::ImmediatePtr(size),
            _ => return None,
        };
        let destination = match (((half & 0x000F) >> 2) as u8) & 0b00000011 {
            0x00 => Operand::Register,
            0x01 => Operand::RegisterPtr,
            // 0x02 is invalid, can't use an immediate value as a destination
            0x03 => Operand::ImmediatePtr(size),
            _ => return None,
        };
        let condition = match (half & 0x00F0) as u8 {
            0x00 => Condition::Always,
            0x10 => Condition::Zero,
            0x20 => Condition::NotZero,
            0x30 => Condition::Carry,
            0x40 => Condition::NotCarry,
            0x50 => Condition::GreaterThan,
            0x60 => Condition::LessThanEqualTo,
            _ => return None,
        };
        match opcode {
            0x00 => Some(Instruction::Nop()),
            0x10 => Some(Instruction::Halt(condition)),
            0x20 => Some(Instruction::Brk(condition)),

            0x01 => Some(Instruction::Add(size, condition, destination, source)),
            0x11 => Some(Instruction::Inc(size, condition, source)),
            0x21 => Some(Instruction::Sub(size, condition, destination, source)),
            0x31 => Some(Instruction::Dec(size, condition, source)),

            0x02 => Some(Instruction::Mul(size, condition, destination, source)),
            0x22 => Some(Instruction::Div(size, condition, destination, source)),
            0x32 => Some(Instruction::Rem(size, condition, destination, source)),

            0x03 => Some(Instruction::And(size, condition, destination, source)),
            0x13 => Some(Instruction::Or(size, condition, destination, source)),
            0x23 => Some(Instruction::Xor(size, condition, destination, source)),
            0x33 => Some(Instruction::Not(size, condition, source)),

            0x04 => Some(Instruction::Sla(size, condition, destination, source)),
            0x24 => Some(Instruction::Rol(size, condition, destination, source)),

            0x05 => Some(Instruction::Sra(size, condition, destination, source)),
            0x15 => Some(Instruction::Srl(size, condition, destination, source)),
            0x25 => Some(Instruction::Ror(size, condition, destination, source)),

            0x06 => Some(Instruction::Bse(size, condition, destination, source)),
            0x16 => Some(Instruction::Bcl(size, condition, destination, source)),
            0x26 => Some(Instruction::Bts(size, condition, destination, source)),

            0x07 => Some(Instruction::Cmp(size, condition, destination, source)),
            0x17 => Some(Instruction::Mov(size, condition, destination, source)),
            0x27 => Some(Instruction::Movz(size, condition, destination, source)),

            0x08 => Some(Instruction::Jmp(condition, source)),
            0x18 => Some(Instruction::Call(condition, source)),
            0x28 => Some(Instruction::Loop(condition, source)),

            0x09 => Some(Instruction::Rjmp(condition, source)),
            0x19 => Some(Instruction::Rcall(condition, source)),
            0x29 => Some(Instruction::Rloop(condition, source)),

            0x39 => Some(Instruction::Rta(condition, destination, source)),

            0x0A => Some(Instruction::Push(size, condition, source)),
            0x1A => Some(Instruction::Pop(size, condition, source)),
            0x2A => Some(Instruction::Ret(condition)),
            0x3A => Some(Instruction::Reti(condition)),

            0x0B => Some(Instruction::In(condition, destination, source)),
            0x1B => Some(Instruction::Out(condition, destination, source)),

            0x0C => Some(Instruction::Ise(condition)),
            0x1C => Some(Instruction::Icl(condition)),

            _ => None,
        }
    }
}
