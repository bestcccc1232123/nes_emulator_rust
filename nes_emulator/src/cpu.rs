/**
 * Simulator of 6502.
 * 
 * For 6502 instruction references, see http://www.obelisk.me.uk/6502/reference.html and http://www.6502.org/tutorials/6502opcodes.html
 */

// OPCODEs.
const OPCODE_BRK : u8 = 0x00;
const OPCODE_INX : u8 = 0xe8;
const OPCODE_LDA_IMMEDIATE : u8 = 0xa9;
const OPCODE_TAX : u8 = 0xaa;

// Status masks.
// Note that we only have 7 status registers for 8 bits of "process status" register.
// Bit 5 is always set to 1. Since nothing can change it, it is of no use to programmers.
//
// See https://www.atarimagazines.com/compute/issue53/047_1_All_About_The_Status_Register.php
const STATUS_MASK_CARRY_FLAG : u8 = 0b0000_0001;  // C bit: bit 0
const STATUS_MASK_ZERO_FLAG: u8 = 0b0000_0010;  // Z bit: bit 1
const STATUS_MASK_INTERRUPT_DISABLE : u8 = 0b0000_0100;  // I bit: bit 2
const STATUS_MASK_DECIMAL_MODE : u8 = 0b0000_1000;  // D bit: bit 3
const STATUS_MASK_BREAK_COMMAND : u8 = 0b0001_0000;  // B bit: bit 4
const STATUS_MASK_OVERFLOW_FLAG : u8 = 0b0100_0000;  // V bit: bit 6
const STATUS_MASK_NEGATIVE_FLAG : u8 = 0b1000_0000;  // N bit: bit 7

// Initial register values.
// Bit 5 in the status register is always set to 1.
const DEFAULT_STATUS_REGISTER : u8 = 0b0001_0000;  // A default 


pub struct CPU {
    pub reg_a: u8,      // register A.
    pub reg_x: u8,      // register X.
    pub reg_status: u8, // program status register.
    pub pc: u16,        // program counter.
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            reg_a: 0,
            reg_x: 0,
            reg_status: DEFAULT_STATUS_REGISTER,
            pc: 0,
        }
    }

    // TODO: abstract the program into a separate interface.
    pub fn interpret(&mut self, program: &Vec<u8>) {
        self.pc = 0;

        loop {
            let op_code = self.read_and_inc_pc(program);
            match op_code {
                OPCODE_BRK => {
                    return;
                }
                OPCODE_INX => {
                    self.inx();
                }
                OPCODE_LDA_IMMEDIATE => {
                    let val = self.read_and_inc_pc(program);
                    self.lda(val);
                }
                OPCODE_TAX => {
                    self.tax();
                }
                _ => todo!(""),
            }
        }
    }

    // Returns the program pointed by |pc| and increment |pc|.
    fn read_and_inc_pc(&mut self, program: &Vec<u8>) -> u8 {
        let opcode = program[self.pc as usize];
        self.pc += 1;
        opcode
    }

    // Sets the N bit of status register based on the value of |register|.
    fn set_negative_flag(&mut self, register : u8) {
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
    fn test_initial_register() {
        let cpu = CPU::new();
    
        assert_eq!(cpu.pc, 0);
        assert_eq!(cpu.reg_a, 0);
        assert_eq!(cpu.reg_status, 0b0001_0000);
    }

    #[test]
    fn test_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        let program = vec![0xa9, 0b0000_1111, 0x00];

        cpu.interpret(&program);

        assert_eq!(cpu.pc, 3);
        assert_eq!(cpu.reg_a, 0b0000_1111);
        assert_eq!(cpu.reg_status, 0b0001_0000);
    }

    #[test]
    fn test_lda_immediate_negative_flag() {
        let mut cpu = CPU::new();
        let program = vec![0xa9, 0b1000_1111, 0x00];

        cpu.interpret(&program);

        assert_eq!(cpu.pc, 3);
        assert_eq!(cpu.reg_a, 0b1000_1111);
        assert_eq!(cpu.reg_status, 0b1001_0000);
    }

    #[test]
    fn test_lda_immediate_zero_flag() {
        let mut cpu = CPU::new();
        let program = vec![0xa9, 0x00, 0x00];

        cpu.interpret(&program);

        assert_eq!(cpu.pc, 3);
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

        cpu.interpret(&program);

        assert_eq!(cpu.pc, 4);
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

        cpu.interpret(&program);

        assert_eq!(cpu.pc, 4);
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

        cpu.interpret(&program);

        assert_eq!(cpu.pc, 4);
        assert_eq!(cpu.reg_a, 0x00);
        assert_eq!(cpu.reg_x, 0x00);
        assert_eq!(cpu.reg_status, 0b0001_0010);
    }

    #[test]
    fn test_inx() {
        let mut cpu = CPU::new();
        cpu.reg_x = 0x0f;
        // INX
        let program = vec![0xe8, 0x00];
        
        cpu.interpret(&program);

        assert_eq!(cpu.pc, 2);
        assert_eq!(cpu.reg_x, 0x10);
        assert_eq!(cpu.reg_status, 0b0001_0000);
    }

    #[test]
    fn test_inx_zero_flag() {
        let mut cpu = CPU::new();
        cpu.reg_x = 0xff;
        // INX
        let program = vec![0xe8, 0x00];
        
        cpu.interpret(&program);

        assert_eq!(cpu.pc, 2);
        assert_eq!(cpu.reg_x, 0x00);
        assert_eq!(cpu.reg_status, 0b0001_0010);       
    }

    #[test]
    fn test_inx_negative_flag() {
        let mut cpu = CPU::new();
        cpu.reg_x = 0xe0;
        // INX
        let program = vec![0xe8, 0x00];
        
        cpu.interpret(&program);

        assert_eq!(cpu.pc, 2);
        assert_eq!(cpu.reg_x, 0xe1);
        assert_eq!(cpu.reg_status, 0b1001_0000);       
    }

    #[test]
    fn test_inx_overflow() {
        let mut cpu = CPU::new();
        cpu.reg_x = 0xff;
        // INX
        // INX
        let program = vec![0xe8, 0xe8, 0x00];

        cpu.interpret(&program);

        assert_eq!(cpu.reg_x, 1)
    }
}
