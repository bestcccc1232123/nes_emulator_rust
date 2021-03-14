pub struct CPU {
    pub reg_a: u8,      // register A.
    pub reg_status: u8, // program status register.
    pub pc: u16,        // program counter.
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            reg_a: 0,
            reg_status: 0,
            pc: 0,
        }
    }

    pub fn interpret(&mut self, program: Vec<u8>) {
        self.pc = 0;

        loop {
            let op_code = program[self.pc as usize];
            self.pc += 1;
            match op_code {
                0x00 => {
                    // BRK
                    return;
                }
                0xa9 => {
                    // LDA
                    let param = program[self.pc as usize];
                    self.pc += 1;
                    self.reg_a = param;
                    if self.reg_a == 0 {
                        self.reg_status = self.reg_status | 0b0000_0010;
                    } else {
                        self.reg_status = self.reg_status & 0b1111_1101;
                    }
                    if self.reg_a & 0b1000_0000 == 0 {
                        self.reg_status = self.reg_status & 0b0111_1111;
                    } else {
                        self.reg_status = self.reg_status | 0b1000_0000;
                    }
                }
                _ => todo!(""),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_lda_load_data() {
        let mut cpu = CPU::new();
        let program = vec![0xa9, 0x01, 0x00];

        cpu.interpret(program);

        assert_eq!(cpu.pc, 3);
        assert_eq!(cpu.reg_a, 0x01);
        assert_eq!(cpu.reg_status, 0b0000_0000);
    }
}
