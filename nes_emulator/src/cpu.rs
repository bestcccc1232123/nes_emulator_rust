/**
 * Simulator of 6502.
 *
 * For 6502 instruction references, see http://www.obelisk.me.uk/6502/reference.html and http://www.6502.org/tutorials/6502opcodes.html
 */
extern crate simple_error;

use simple_error::SimpleError;
use std::result::Result;

// OPCODEs.
const OPCODE_BRK: u8 = 0x00;
const OPCODE_INX: u8 = 0xe8;
const OPCODE_JMP_ABSOLUTE: u8 = 0x4c;
const OPCODE_LDA_IMMEDIATE: u8 = 0xa9;
const OPCODE_TAX: u8 = 0xaa;

// Status masks.
// Note that we only have 7 status registers for 8 bits of "process status" register.
// Bit 5 is always set to 1. Since nothing can change it, it is of no use to programmers.
//
// See https://www.atarimagazines.com/compute/issue53/047_1_All_About_The_Status_Register.php
const STATUS_MASK_CARRY_FLAG: u8 = 0b0000_0001; // C bit: bit 0
const STATUS_MASK_ZERO_FLAG: u8 = 0b0000_0010; // Z bit: bit 1
const STATUS_MASK_INTERRUPT_DISABLE: u8 = 0b0000_0100; // I bit: bit 2
const STATUS_MASK_DECIMAL_MODE: u8 = 0b0000_1000; // D bit: bit 3
const STATUS_MASK_BREAK_COMMAND: u8 = 0b0001_0000; // B bit: bit 4
const STATUS_MASK_OVERFLOW_FLAG: u8 = 0b0100_0000; // V bit: bit 6
const STATUS_MASK_NEGATIVE_FLAG: u8 = 0b1000_0000; // N bit: bit 7

// Initial register values.
// Bit 5 in the status register is always set to 1.
const INIT_STATUS_REGISTER: u8 = 0b0001_0000;
// NES platform has a special mechanism to mark where the CPU should start the execution.
// Upon inserting a new cartridge, the CPU receives a special signal called "Reset interrupt"
// that instructs CPU to set pc to 0xfffc.
const INIT_PROGRAM_COUNTER_ADDR: u16 = 0xfffc;

// Memory layout.

// Max address.
const MEM_ADDR_MAX: u16 = 0xffff;
const MEM_ADDR_SPACE_SIZE: usize = MEM_ADDR_MAX as usize + 1;
// Program ROM address.
const MEM_PRG_ROM_ADDR_START: u16 = 0x8000;
const MEM_PRG_ROM_ADDR_END: u16 = 0xffff;
const MEM_PRG_ROM_SIZE: usize = (MEM_PRG_ROM_ADDR_END - MEM_PRG_ROM_ADDR_START) as usize + 1;

// Represents the memory of 6502.
struct Mem {
    // The maximum addressable memory is 64KB.
    data: [u8; MEM_ADDR_SPACE_SIZE],
}

impl Mem {
    pub fn new() -> Self {
        Mem {
            data: [0; MEM_ADDR_SPACE_SIZE],
        }
    }
    pub fn read(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }

    // Reads two bytes starting at |addr|. Little endian.
    pub fn read16(&self, addr: u16) -> Result<u16, SimpleError> {
        if addr == MEM_ADDR_MAX {
            return Err(SimpleError::new(format!(
                "cannot read two bytes starting from address 0x{:x}",
                MEM_ADDR_MAX
            )));
        }

        let lo = self.read(addr) as u16;
        let hi = self.read(addr + 1) as u16;

        Ok((hi << 8) | lo)
    }
    pub fn write(&mut self, addr: u16, val: u8) {
        self.data[addr as usize] = val;
    }

    pub fn write16(&mut self, addr: u16, val: u16) -> Result<(), SimpleError> {
        if addr == MEM_ADDR_MAX {
            return Err(SimpleError::new(format!(
                "cannot write two bytes at address 0x{:x}",
                MEM_ADDR_MAX
            )));
        }

        let lo = val as u8;
        self.write(addr, lo);

        let hi = (val >> 8) as u8;
        self.write(addr + 1, hi);

        Ok(())
    }

    pub fn write_range(&mut self, start_addr: u16, val: &[u8]) -> Result<(), SimpleError> {
        if start_addr as usize + val.len() > self.data.len() {
            return Err(SimpleError::new(format!(
                "Range exceeds the memory space: start_addr = 0x{:x}, range_length = {}",
                start_addr,
                val.len()
            )));
        }

        for i in 0..val.len() {
            self.write(start_addr + (i as u16), val[i]);
        }
        Ok(())
    }
}

pub struct CPU {
    pub reg_a: u8,      // register A.
    pub reg_x: u8,      // register X.
    pub reg_status: u8, // program status register.
    pub pc: u16,        // program counter.
    mem: Mem,           // Memory.
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            reg_a: 0,
            reg_x: 0,
            reg_status: INIT_STATUS_REGISTER,
            pc: 0,
            mem: Mem::new(),
        }
    }

    // Loads the program into PRG ROM.
    pub fn load(&mut self, program: &[u8]) -> Result<(), SimpleError> {
        self.mem.write_range(MEM_PRG_ROM_ADDR_START, program)?;
        self.mem.write16(INIT_PROGRAM_COUNTER_ADDR, MEM_PRG_ROM_ADDR_START)
    }

    // NES platform has a special mechanism to mark where the CPU should start the execution. Upon inserting a new cartridge, the CPU receives a special signal called "Reset interrupt" that instructs CPU to:
    // 1) reset the state (registers and flags);
    // 2) set program_counter to the 16-bit address that is stored at 0xFFFC.
    pub fn reset(&mut self) {
        self.reg_a = 0;
        self.reg_x = 0;
        self.reg_status = INIT_STATUS_REGISTER;
        self.pc = self.mem.read16(INIT_PROGRAM_COUNTER_ADDR).unwrap();
    }

    // Runs the program started at PRG ROM.
    pub fn run(&mut self) {
        self.pc = MEM_PRG_ROM_ADDR_START;

        loop {
            if !self.step() {
                break;
            }
        }
    }

    // Executes the next instruction, return true to continue.
    pub fn step(&mut self) -> bool {
        let op_code = self.read_and_inc_pc();
        match op_code {
            OPCODE_BRK => {
                return false;
            }
            OPCODE_INX => {
                self.inx();
            }
            OPCODE_JMP_ABSOLUTE => {
                let val = self.read16_and_inc_pc();
                self.jmp(val);
            }
            OPCODE_LDA_IMMEDIATE => {
                let val = self.read_and_inc_pc();
                self.lda(val);
            }
            OPCODE_TAX => {
                self.tax();
            }
            _ => panic!(todo!("")),
        }
        true
    }

    pub fn interpret(&mut self, program: &[u8]) -> Result<(), SimpleError> {
        self.load(program)?;
        self.reset();
        self.run();

        Ok(())
    }

    // Returns the program pointed by |pc| and increment |pc|.
    fn read_and_inc_pc(&mut self) -> u8 {
        let opcode = self.mem.read(self.pc);
        self.pc += 1;
        opcode
    }

    fn read16_and_inc_pc(&mut self) -> u16 {
        let val = self.mem.read16(self.pc).unwrap();
        self.pc += 2;
        val
    }

    // Sets the N bit of status register based on the value of |register|.
    fn set_negative_flag(&mut self, register: u8) {
        if register & 0b1000_0000 == 0 {
            self.reg_status &= !STATUS_MASK_NEGATIVE_FLAG;
        } else {
            self.reg_status |= STATUS_MASK_NEGATIVE_FLAG;
        }
    }

    // Sets the Z bit of status register based on the value of |register|.
    fn set_zero_flag(&mut self, register: u8) {
        if register == 0 {
            self.reg_status |= STATUS_MASK_ZERO_FLAG;
        } else {
            self.reg_status &= !STATUS_MASK_ZERO_FLAG;
        }
    }

    // Handles instruction INX.
    fn inx(&mut self) {
        let (val_x, overflow) = self.reg_x.overflowing_add(1);
        self.reg_x = val_x;

        self.set_negative_flag(self.reg_x);
        self.set_zero_flag(self.reg_x);
    }

    fn jmp(&mut self, addr: u16) {
        self.pc = addr;
    }

    // Handles instruction LDA.
    fn lda(&mut self, val: u8) {
        self.reg_a = val;

        self.set_negative_flag(self.reg_a);
        self.set_zero_flag(self.reg_a);
    }

    // Handles instruction TAX.
    fn tax(&mut self) {
        self.reg_x = self.reg_a;

        self.set_negative_flag(self.reg_x);
        self.set_zero_flag(self.reg_x);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_mem_init() {
        let mem = Mem::new();

        for i in 0..0xffff {
            assert_eq!(mem.read(i as u16), 0x00);
        }
    }

    #[test]
    fn test_mem_read_write() {
        let mut mem = Mem::new();

        mem.write(0x01, 0xff);

        assert_eq!(mem.read(0x01), 0xff);
    }

    #[test]
    fn test_mem_read16() {
        let mut mem = Mem::new();

        mem.write(0x01, 0xff);
        mem.write(0x02, 0xcc);

        assert_eq!(mem.read16(0x01), Ok(0xccff));
    }

    #[test]
    fn test_mem_read16_out_of_range() {
        let mut mem = Mem::new();

        assert_eq!(
            mem.read16(0xffff),
            Err(SimpleError::new(
                "cannot read two bytes starting from address 0xffff"
            ))
        );
    }

    #[test]
    fn test_mem_write_range() {
        let mut mem = Mem::new();
        let input: Vec<u8> = vec![0, 1, 2, 3, 4, 5];

        assert_eq!(mem.write_range(0x01, &input[1..]), Ok(()));

        assert_eq!(mem.read(0x01), 1);
        assert_eq!(mem.read(0x02), 2);
        assert_eq!(mem.read(0x03), 3);
        assert_eq!(mem.read(0x04), 4);
        assert_eq!(mem.read(0x05), 5);
    }

    #[test]
    fn test_mem_write16() {
        let mut mem = Mem::new();

        assert_eq!(mem.write16(0x01, 0xffcc), Ok(()));

        assert_eq!(mem.read16(0x01), Ok(0xffcc));
    }

    #[test]
    fn test_mem_write16_out_or_range() {
        let mut mem = Mem::new();

        assert_eq!(
            mem.write16(0xffff, 0xffff),
            Err(SimpleError::new("cannot write two bytes at address 0xffff"))
        );
    }

    #[test]
    fn test_mem_write_range_out_of_range() {
        let mut mem = Mem::new();
        let input: Vec<u8> = vec![0, 1, 2, 3, 4, 5];

        assert_eq!(
            mem.write_range(0xfffe, &input[1..]),
            Err(SimpleError::new(
                "Range exceeds the memory space: start_addr = 0xfffe, range_length = 5"
            ))
        );
    }

    #[test]
    fn test_initial_register() {
        let mut cpu = CPU::new();
        cpu.reset();

        assert_eq!(cpu.reg_a, 0);
        assert_eq!(cpu.reg_x, 0);
        assert_eq!(cpu.reg_status, 0b0001_0000);
        assert_eq!(cpu.pc, 0x00);
    }

    #[test]
    fn test_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        let program = vec![0xa9, 0b0000_1111, 0x00];

        assert_eq!(cpu.interpret(&program), Ok(()));

        assert_eq!(cpu.reg_a, 0b0000_1111);
        assert_eq!(cpu.reg_status, 0b0001_0000);
    }

    #[test]
    fn test_lda_immediate_negative_flag() {
        let mut cpu = CPU::new();
        let program = vec![0xa9, 0b1000_1111, 0x00];

        assert_eq!(cpu.interpret(&program), Ok(()));

        assert_eq!(cpu.reg_a, 0b1000_1111);
        assert_eq!(cpu.reg_status, 0b1001_0000);
    }

    #[test]
    fn test_lda_immediate_zero_flag() {
        let mut cpu = CPU::new();
        let program = vec![0xa9, 0x00, 0x00];

        assert_eq!(cpu.interpret(&program), Ok(()));

        assert_eq!(cpu.reg_a, 0x00);
        assert_eq!(cpu.reg_status, 0b0001_0010);
    }

    #[test]
    fn test_tax_load_data() {
        let mut cpu = CPU::new();
        // LDA #$8f
        // TAX
        // BRK
        let program = vec![0xa9, 0b0111_1111, 0xaa, 0x00];

        assert_eq!(cpu.interpret(&program), Ok(()));

        assert_eq!(cpu.reg_a, 0b0111_1111);
        assert_eq!(cpu.reg_x, 0b0111_1111);
        assert_eq!(cpu.reg_status, 0b0001_0000);
    }

    #[test]
    fn test_tax_negative_flag() {
        let mut cpu = CPU::new();
        // LDA #$ff
        // TAX
        // BRK
        let program = vec![0xa9, 0b1111_1111, 0xaa, 0x00];

        assert_eq!(cpu.interpret(&program), Ok(()));

        assert_eq!(cpu.reg_a, 0b1111_1111);
        assert_eq!(cpu.reg_x, 0b1111_1111);
        assert_eq!(cpu.reg_status, 0b1001_0000);
    }

    #[test]
    fn test_tax_zero_flag() {
        let mut cpu = CPU::new();
        // LDA #$ff
        // TAX
        // BRK
        let program = vec![0xa9, 0x00, 0xaa, 0x00];

        assert_eq!(cpu.interpret(&program), Ok(()));

        assert_eq!(cpu.reg_a, 0x00);
        assert_eq!(cpu.reg_x, 0x00);
        assert_eq!(cpu.reg_status, 0b0001_0010);
    }

    #[test]
    fn test_inx() {
        let mut cpu = CPU::new();
        // INX
        // INX
        let program = vec![0xe8, 0xe8, 0x00];

        assert_eq!(cpu.interpret(&program), Ok(()));

        assert_eq!(cpu.reg_x, 0x02);
        assert_eq!(cpu.reg_status, 0b0001_0000);
    }

    #[test]
    fn test_inx_zero_flag() {
        let mut cpu = CPU::new();
        let mut program = vec![0; 8000];
        for i in 0..0x100 {
            program[i] = 0xe8;
        }

        // INX * 256
        // BRK
        assert_eq!(cpu.interpret(&program), Ok(()));

        assert_eq!(cpu.reg_x, 0x00);
        assert_eq!(cpu.reg_status, 0b0001_0010);
    }

    #[test]
    fn test_inx_negative_flag() {
        let mut cpu = CPU::new();
        let mut program = vec![0; 8000];
        for i in 0..0xf0 {
            program[i] = 0xe8;
        }
        // INX * 0xf0
        // BRK
        assert_eq!(cpu.interpret(&program), Ok(()));

        assert_eq!(cpu.reg_x, 0xf0);
        assert_eq!(cpu.reg_status, 0b1001_0000);
    }

    #[test]
    fn test_inx_overflow() {
        let mut cpu = CPU::new();
        let mut program = vec![0; 8000];
        for i in 0..0x101 {
            program[i] = 0xe8;
        }

        // INX * 257
        // BRK
        assert_eq!(cpu.interpret(&program), Ok(()));

        assert_eq!(cpu.reg_x, 1)
    }
}
