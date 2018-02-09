use std::collections::VecDeque;
use platform::Platform;

pub struct DebugFrame {
    pc: u16,
    op: u8,
    a: u8,
    x: u8,
    y: u8,
    registers : u8,
    opcode_name : String
}

pub struct MOS6502 {
    reg_a: u8,
    reg_x: u8,
    reg_y: u8,
    reg_sp: u8,
    reg_pc: u16,
    f_negative: bool,
    f_overflow: bool,
    f_constant: bool,
    f_break: bool,
    f_decimal: bool,
    f_interrupt: bool,
    f_zero: bool,
    f_carry: bool,

    cycle_count: i32,
    is_stopped: bool,

    debug_vector : VecDeque<DebugFrame>,
    platform : Box<Platform>
}

impl MOS6502 {
    pub fn new(platform : Box<Platform>) -> MOS6502 {
        MOS6502 {
            reg_a: 0,
            reg_x: 0,
            reg_y: 0,
            reg_sp: 0xff,
            reg_pc: 0,
            f_negative: false,
            f_overflow: false,
            f_constant: true,
            f_break: true,
            f_decimal: false,
            f_interrupt: false,
            f_zero: false,
            f_carry: false,
            cycle_count: 0,
            is_stopped: false,
            platform: platform,
            debug_vector :  VecDeque::new()
        }
    }

    pub fn reset(&mut self) {
            self.reg_a = 0;
            self.reg_x = 0;
            self.reg_y = 0;
            self.reg_sp = 0xfd;
            self.f_negative = false;
            self.f_overflow = false;
            self.f_constant = true;
            self.f_break = true;
            self.f_decimal = false;
            self.f_interrupt = false;
            self.f_zero = false;
            self.f_carry = false;
            self.is_stopped = false;

            let lo = self.platform.read(0xfffc) as u16;
            let hi = self.platform.read(0xfffd) as u16;

            self.reg_pc = lo + (hi << 8);
            self.cycle_count = 0;
    }

    fn read_pc(&mut self) -> u8 {
        let addr = self.reg_pc;
        let ret = self.read_u8(addr);
        self.reg_pc += 1;
        ret
    }

    pub fn read_u8(&mut self, address: u16) -> u8 {
        self.platform.read(address)
    }

    pub fn key_ready(&self) -> bool {
        self.platform.key_ready()
    }

    pub fn key_pressed(&mut self, key: u8) {
        self.platform.key_pressed(key);
    }

    pub fn write_u8(&mut self, address: u16, value: u8) {
        self.platform.write(address, value);
    }

    fn nop(&mut self) {
        self.cycles(2);
    }

    fn cycles(&mut self, num_cycles: i32) {
        self.cycle_count += num_cycles;
    }

    fn update_flags_zn(&mut self, value: u8) {
        self.f_zero = value == 0;
        self.f_negative = (value & 0x80) == 0x80;
    }

    fn get_absolute_addr(&mut self, offset: u8) -> u16 {
        let lo = self.read_pc() as u16;
        let mut hi = self.read_pc() as u16;
        hi = hi << 8;
        lo + hi + (offset as u16)
    }

    fn get_zeropage_addr(&mut self, offset: u8) -> u16 {
        let addr = self.read_pc() as u16;
        addr + (offset as u16)
    }

    fn get_indirect_addr(&mut self, index: u16) -> u16 {
        let lo = self.read_u8(index) as u16;
        let hi = (self.read_u8(index + 1) as u16) << 8;
        lo + hi
    }

    fn get_indirect_x_addr(&mut self) -> u16 {
        let offset = self.reg_x;
        let index = (offset as u16) + (self.read_pc() as u16);
        self.get_indirect_addr(index)
    }

    fn get_indirect_y_addr(&mut self) -> u16 {
        let offset = self.reg_y;
        let index = self.read_pc() as u16;
        self.get_indirect_addr(index) + (offset as u16)
    }

    fn stack_push(&mut self, value: u8) {
        let addr = 0x100 + (self.reg_sp as u16) as u16;
        self.write_u8(addr, value);
        self.reg_sp = self.reg_sp - 1;
        if self.reg_sp <= 1 {
            println!("push: stack overflow {:?}", self.reg_pc);
            self.is_stopped = true;
        }
    }

    fn stack_pull(&mut self) -> u8 {
        if self.reg_sp >= 0xff {
            println!("pull: stack overflow {:?}", self.reg_pc);
            self.is_stopped = true;
        }
        self.reg_sp = self.reg_sp + 1;
        let addr = 0x100 + (self.reg_sp as u16) as u16;
        let result = self.read_u8(addr);
        result
    }

    fn set_status_registers(&mut self, value: u8) {
        self.f_negative = (value & 0x80) == 0x80;
        self.f_overflow = (value & 0x40) == 0x40;
        self.f_constant = true;
        self.f_break = (value & 0x10) == 0x10;
        self.f_decimal = (value & 0x08) == 0x08;
        self.f_interrupt = (value & 0x04) == 0x04;
        self.f_zero = (value & 0x02) == 0x02;
        self.f_carry = (value & 0x01) == 0x01;
    }

    fn get_status_registers(&mut self) -> u8 {
        (if self.f_negative { 0x80 } else { 0 }) |
        (if self.f_overflow { 0x40 } else { 0 }) |
        (0x20) |
        (if self.f_break {0x10} else { 0 }) |
        (if self.f_decimal { 0x08 } else { 0 })  |
        (if self.f_interrupt { 0x04 } else { 0 })  |        
        (if self.f_zero { 0x02 } else { 0 })  |
        (if self.f_carry { 0x01 } else { 0 })
    }

    fn get_carry_amount(&mut self) -> u8 {
        if self.f_carry { 1 } else { 0 }
    }

    fn update_flags_zcn(&mut self, result: i32) {
        self.f_negative = (result & 0x80) == 0x80;
        self.f_zero = (result as u8) == 0;
        self.f_carry = result >= 0;
        //self.f_carry = sum & 0xff00 > 0;
    }

    fn branch(&mut self) {
        let addr = self.reg_pc;
        let offset = self.read_pc() as u16;
        if offset > 0x7F {
            self.reg_pc = addr - (0xff - offset);
        }
        else {
            self.reg_pc += offset;
        }
    }
    
    fn adc(&mut self, value: u8) {
        let sum = (self.reg_a as u16) + (value as u16) + (self.get_carry_amount() as u16);
        let result = sum as u8;
        self.f_carry = sum & 0xff00 > 0;
        self.f_overflow = (self.reg_a ^ value) & 0x80 == 0 && (self.reg_a ^ result) & 0x80 == 0x80;
        self.reg_a = result;
        self.update_flags_zn(result);
    }

    fn sbc(&mut self, value: u8) {
        let borrow = if self.f_carry { 1 } else { 0 };
        let complement = 255 - value;
        let sum = (self.reg_a as u16) + (complement as u16) + (borrow as u16);
        let result = sum as u8;
        self.f_carry = sum & 0xff00 > 0;
        self.f_overflow = (self.reg_a ^ complement) & 0x80 == 0 && (self.reg_a ^ result) & 0x80 == 0x80;
        self.reg_a = result;
        self.update_flags_zn(result);
    }

    fn ror(&mut self, value: u8) -> u8 {
        let carry = if self.f_carry { 0x80 } else { 0 };
        self.f_carry = (value & 0x01) == 0x01;
        let result = carry | (value >> 1);
        self.update_flags_zn(result as u8);
        result as u8
    }

    fn rol(&mut self, value: u8) -> u8 {
        let carry = if self.f_carry { 0x01 } else { 0 };
        let result = (value as u16) << 1 | carry;
        self.f_carry = (result & 0xff00) > 0;
        self.update_flags_zn(result as u8);        
        result as u8
    }

    fn asl(&mut self, value: u8) -> u8 {
        let shift = (value as u16) << 1;
        self.f_carry = shift & 0xff00 > 0;
        self.update_flags_zn(shift as u8);        
        shift as u8
    }

    fn lsr(&mut self, value: u8) -> u8 {
        self.f_carry = (value & 0x01) == 0x01;
        let result = value >> 1;
        self.update_flags_zn(result);
        result
    }

    pub fn get_cycle_count(&mut self) -> i32 {
        self.cycle_count
    }

    pub fn is_running(&mut self) -> bool {
        !self.is_stopped
    }

    pub fn run(&mut self, target_cycles: i32) -> i32 {
        self.cycle_count = 0;
        while self.cycle_count < target_cycles && !self.is_stopped {
                self.step();
        } 
     
        self.cycle_count
    }

    pub fn step(&mut self) {
        let _starting_pc = self.reg_pc;
        let mut opcode_name = String::new();
        let opcode = self.read_pc();
        match opcode {
            0x69 => {
                //ADC,IMM,2,2,CZidbVN
                opcode_name = String::from("ADC");
                let value = self.read_pc();
                self.adc(value);
                self.cycles(2);
            }
            0x65 => {
                //ADC,ZP,2,3,CZidbVN
                opcode_name = String::from("ADC");
                let addr = self.get_zeropage_addr(0);
                let value = self.read_u8(addr);
                self.adc(value);
                self.cycles(3);
            }
            0x75 => {
                //ADC,ZPX,2,4,CZidbVN
                opcode_name = String::from("ADC");
                let offset = self.reg_x;
                let addr = self.get_zeropage_addr(offset);
                let value = self.read_u8(addr);
                self.adc(value);
                self.cycles(4);
            }
            0x6d => {
                //ADC,ABS,3,4,CZidbVN
                opcode_name = String::from("ADC");
                let addr = self.get_absolute_addr(0);
                let value = self.read_u8(addr);
                self.adc(value);
                self.cycles(4);
            }
            0x7d => {
                //ADC,ABSX,3,4,CZidbV
                opcode_name = String::from("ADC");                
                let offset = self.reg_x;
                let addr = self.get_absolute_addr(offset);
                let value = self.read_u8(addr);
                self.adc(value);
                self.cycles(4);
            }
            0x79 => {
                //ADC,ABSY,3,4,CZidbV
                opcode_name = String::from("ADC");                
                let offset = self.reg_y;
                let addr = self.get_absolute_addr(offset);
                let value = self.read_u8(addr);
                self.adc(value);
                self.cycles(4);
            }
            0x61 => {
                //ADC,INDX,2,6,CZidbV
                opcode_name = String::from("ADC");                
                let addr = self.get_indirect_x_addr();
                let value = self.read_u8(addr);
                self.adc(value);
                self.cycles(6);
            }
            0x71 => {
                //ADC,INDY,2,5,CZidbV
                opcode_name = String::from("ADC");                
                let addr = self.get_indirect_y_addr();
                let value = self.read_u8(addr);
                self.adc(value);
                self.cycles(5);
            }
            0x29 => {
                //AND,IMM,2,2,cZidbvN
                opcode_name = String::from("AND");
                let value = self.read_pc();
                let result = self.reg_a & value;
                self.reg_a = result;
                self.update_flags_zn(result);
                self.cycles(2);
            }
            0x25 => {
                //AND,ZP,2,3,cZidbvN
                opcode_name = String::from("AND");                
                let address = self.get_zeropage_addr(0);
                let value = self.read_u8(address);
                let result = self.reg_a & value;
                self.reg_a = result;
                self.update_flags_zn(result);
                self.cycles(3);
            }
            0x35 => {
                //AND,ZPX,2,4,cZidbvN
                opcode_name = String::from("AND");                
                let offset = self.reg_x;
                let address = self.get_zeropage_addr(offset);
                let value = self.read_u8(address);
                let result = self.reg_a & value;
                self.reg_a = result;
                self.update_flags_zn(result);
                self.cycles(3);
            }
            0x2d => {
                //AND,ABS,3,4,cZidbVN
                opcode_name = String::from("AND");                
                let address = self.get_absolute_addr(0);
                let value = self.read_u8(address);
                let result = self.reg_a & value;
                self.reg_a = result;
                self.update_flags_zn(result);
                self.cycles(3);
            }
            0x3d => {
                //AND,ABSX,3,4,cZidbv
                opcode_name = String::from("AND");
                let offset = self.reg_x;
                let address = self.get_absolute_addr(offset);
                let value = self.read_u8(address);
                let result = self.reg_a & value;
                self.reg_a = result;
                self.update_flags_zn(result);
                self.cycles(4);
            }
            0x39 => {
                //AND,ABSY,3,4,cZidbv
                opcode_name = String::from("AND");
                let offset = self.reg_y;
                let address = self.get_absolute_addr(offset);
                let value = self.read_u8(address);
                let result = self.reg_a & value;
                self.reg_a = result;
                self.update_flags_zn(result);
                self.cycles(4);
            }
            0x21 => {
                //AND,INDX,2,6,cZidbv
                opcode_name = String::from("AND");                
                let address = self.get_indirect_x_addr();
                let value = self.read_u8(address);
                let result = self.reg_a & value;
                self.reg_a = result;
                self.update_flags_zn(result);
                self.cycles(6);
            }
            0x31 => {
                //AND,INDY,2,5,cZidbv
                opcode_name = String::from("AND");                
                let address = self.get_indirect_y_addr();
                let value = self.read_u8(address);
                let result = self.reg_a & value;
                self.reg_a = result;
                self.update_flags_zn(result);
                self.cycles(5);
            }
            0x0a => {
                //ASL,ACC,1,2,CZidbvN
                opcode_name = String::from("ASL");
                let value = self.reg_a;
                self.reg_a = self.asl(value);
                self.cycles(2);
            }
            0x06 => {
                //ASL,ZP,2,5,CZidbvN
                opcode_name = String::from("ASL");
                let addr = self.get_zeropage_addr(0);
                let mut value = self.read_u8(addr);
                value = self.asl(value);
                self.write_u8(addr, value);
                self.cycles(5);
            }
            0x16 => {
                //ASL,ZPX,2,6,CZidbvN
                opcode_name = String::from("ASL");                
                let offset = self.reg_x;
                let addr = self.get_zeropage_addr(offset);
                let mut value = self.read_u8(addr);
                value = self.asl(value);
                self.write_u8(addr, value);
                self.cycles(5);
            }
            0x0e => {
                //ASL,ABS,3,6,CZidbvN
                opcode_name = String::from("ASL");                
                let addr = self.get_absolute_addr(0);
                let mut value = self.read_u8(addr);
                value = self.asl(value);
                self.write_u8(addr, value);
                self.cycles(5);
            }
            0x1e => {
                //ASL,ABSX,3,7,CZidbv
                opcode_name = String::from("ASL");
                let offset = self.reg_x;
                let addr = self.get_absolute_addr(offset);
                let mut value = self.read_u8(addr);
                value = self.asl(value);
                self.write_u8(addr, value);
                self.cycles(5);
            }
            0x90 => {
                //BCC,REL,2,2/3,czidb
                opcode_name = String::from("BCC");
                if !self.f_carry {
                    self.branch();
                    self.cycles(3);
                } else {
                    self.read_pc();
                    self.cycles(2);
                }
            }
            0xB0 => {
                //BCS,REL,2,2/3,czidb
                opcode_name = String::from("BCS");                
                if self.f_carry {
                    self.branch();
                    self.cycles(3);
                } else {
                    self.read_pc();
                    self.cycles(2);
                }                        
            }
            0xF0 => {
                //BEQ,REL,2,2/3,czidb
                opcode_name = String::from("BEQ");
                if self.f_zero {
                    self.branch();
                    self.cycles(3);
                } else {
                    self.read_pc();                            
                    self.cycles(2);
                }
            }
            0x30 => {
                //BMI,REL,2,2/3,czidb
                opcode_name = String::from("BMI");                
                if self.f_negative {
                    self.branch();
                    self.cycles(3);
                } else {
                    self.read_pc();                            
                    self.cycles(2);
                }
            }
            0xD0 => {
                //BNE,REL,2,2/3,czidb
                opcode_name = String::from("BNE");                
                if !self.f_zero {
                    self.branch();
                    self.cycles(3);
                } else {
                    self.read_pc();                            
                    self.cycles(2);
                }
            }
            0x10 => {
                //BPL,REL,2,2/3,czidb
                opcode_name = String::from("BPL");                                
                if !self.f_negative {
                    self.branch();
                    self.cycles(3);
                } else {
                    self.read_pc();                            
                    self.cycles(2);
                }
            }
            0x50 => {
                //BVC,REL,2,2/3,czidb
                opcode_name = String::from("BVC");                                
                if !self.f_overflow {
                    self.branch();
                    self.cycles(3);
                } else {
                    self.read_pc();                            
                    self.cycles(2);
                }
            }
            0x70 => {
                //BVS,REL,2,2/3,czidb
                opcode_name = String::from("BVS");                                
                if self.f_overflow {
                    self.branch();
                    self.cycles(3);
                } else {
                    self.read_pc();                            
                    self.cycles(2);
                }
            }
            0x24 => {
                //BIT,ZP,2,3,cZidbVN
                opcode_name = String::from("BIT");
                let addr = self.get_zeropage_addr(0);
                let value = self.read_u8(addr);
                let result = self.reg_a & value;
                self.update_flags_zn(result);
                self.f_overflow = (value & 0x40) == 0x40; 
                self.cycles(3);
            }
            0x2c => {
                //BIT,ABS,3,4,cZidbVN
                opcode_name = String::from("BIT");
                let addr = self.get_absolute_addr(0);
                let value = self.read_u8(addr);
                let result = self.reg_a & value;
                self.f_overflow = (value & 0x40) == 0x40; 
                self.update_flags_zn(result);
                self.cycles(4);
            }
            0x00 => {
                opcode_name = String::from("BRK");
                //BRK,IMP,1,7,czidbVN
                self.f_break = true;
                self.cycles(2);
                println!("Stopping execution on BRK {:04x}", self.reg_pc);
                self.is_stopped = true;
            }
            0x18 => {
                //CLC,IMP,1,2,CzidbVN
                opcode_name = String::from("CLC");
                self.f_carry = false;
                self.cycles(2);
            }
            0xd8 => {
                //CLD,IMP,1,2,czidbVN
                opcode_name = String::from("CLD");
                self.f_decimal = false;
                self.cycles(2);
            }
            0x58 => {
                //CLI,IMP,1,2,czIdbVN
                opcode_name = String::from("CLI");
                self.f_interrupt = false;
                self.cycles(2);
            }
            0xb8 => {
                //CLV,IMP,1,2,czidbVN
                opcode_name = String::from("CLV");
                self.f_overflow = false;
                self.cycles(2);
            }
            0xea => {
                //NOP,IMP,1,2,czidbVN
                opcode_name = String::from("NOP");
                self.nop()
            }
            0x48 => {
                //PHA,IMP,1,3,czidbVN
                opcode_name = String::from("PHA");
                let value = self.reg_a;
                self.stack_push(value);
                self.cycles(3);
            }
            0x68 => {
                //PLA,IMP,1,4,cZidbVN
                opcode_name = String::from("PLA");
                let value = self.stack_pull();
                self.reg_a = value;
                self.update_flags_zn(value);
                self.cycles(4);
            }
            0x08 => {
                //PHP,IMP,1,3,
                opcode_name = String::from("PHP");
                let value = self.get_status_registers();
                self.stack_push(value);
                self.cycles(3);
            }
            0x28 => {
                //PLP,IMP,1,4,CZIdbVN
                opcode_name = String::from("PLP");
                let value = self.stack_pull();
                self.set_status_registers(value);
                self.cycles(4);
            }
            0x40 => {
                //RTI,IMP,1,6,czidbVN
                opcode_name = String::from("RTI");
                let psw = self.stack_pull();
                self.set_status_registers(psw);
                let mut addr = self.stack_pull() as u16;
                addr |= (self.stack_pull() as u16) << 8;
                self.reg_pc = addr;
                self.cycles(6);
            }
            0x60 => {
                //RTS,IMP,1,6,czidbVN
                opcode_name = String::from("RTS");
                let mut addr = self.stack_pull() as u16;
                addr |= (self.stack_pull() as u16) << 8;
                self.reg_pc = addr;
                self.cycles(2);                        
            }
            0x38 => {
                //SEC,IMP,1,2,CzidbVN
                opcode_name = String::from("SEC");
                self.f_carry = true;
                self.cycles(2);
            }
            0xf8 => {
                //SED,IMP,1,2,czidbVN
                opcode_name = String::from("SED");
                self.f_decimal = true;
                self.cycles(2);
            }
            0x78 => {
                //SEI,IMP,1,2,czIdbVN
                opcode_name = String::from("SEI");
                self.f_interrupt = true;
                self.cycles(2);
            }
            0xaa => {
                //TAX,IMP,1,2,cZidbVN
                opcode_name = String::from("TAX");
                let value = self.reg_a;
                self.update_flags_zn(value);
                self.reg_x = value;
                self.cycles(2)
            }
            0x8a => {
                //TXA,IMP,1,2,cZidbVN
                opcode_name = String::from("TXA");
                let value = self.reg_x;
                self.update_flags_zn(value);
                self.reg_a = value;
                self.cycles(2)
            }
            0xa8 => {
                //TAY,IMP,1,2,cZidbVN
                opcode_name = String::from("TAY");
                let value = self.reg_a;
                self.update_flags_zn(value);
                self.reg_y = value;
                self.cycles(2)
            }
            0x98 => {
                //TYA,IMP,1,2,cZidbVN
                opcode_name = String::from("TYA");
                let value = self.reg_y;
                self.update_flags_zn(value);
                self.reg_a = value;
                self.cycles(2)
            }
            0xba => {
                //TSX,IMP,1,2,cZidbVN
                opcode_name = String::from("TSX");
                let value = self.reg_sp;
                self.update_flags_zn(value);
                self.reg_x = value;
            }
            0x9a => {
                //TXS,IMP,1,2,czidbVN
                opcode_name = String::from("TXS");
                let value = self.reg_x;
                self.update_flags_zn(value);
                self.reg_sp = value;
            }
            0xc9 => {
                //CMP,IMM,2,2,CZidbVN
                opcode_name = String::from("CMP");
                let value = self.read_pc();
                let result = (self.reg_a as i32) - (value as i32);
                self.update_flags_zcn(result);
                self.cycles(2);
            }
            0xc5 => {
                //CMP,ZP,2,3,CZidbVN
                opcode_name = String::from("CMP");
                let addr = self.get_zeropage_addr(0);
                let value = self.read_u8(addr);
                let result = (self.reg_a as i32) - (value as i32);
                self.update_flags_zcn(result);
                self.cycles(3);
            }
            0xd5 => {
                //CMP,ZPX,2,4,CZidbVN
                opcode_name = String::from("CMP");
                let offset = self.reg_x;
                let addr = self.get_zeropage_addr(offset);
                let value = self.read_u8(addr);
                let result = (self.reg_a as i32) - (value as i32);
                self.update_flags_zcn(result);
                self.cycles(4);
            }
            0xcd => {
                //CMP,ABS,3,4,CZidbVN
                opcode_name = String::from("CMP");
                let addr = self.get_absolute_addr(0);
                let value = self.read_u8(addr);
                let result = (self.reg_a as i32) - (value as i32);
                self.update_flags_zcn(result);
                self.cycles(4);
            }
            0xdd => {
                //CMP,ABSX,3,4,CZidbv
                opcode_name = String::from("CMP");
                let offset = self.reg_x;
                let addr = self.get_absolute_addr(offset);
                let value = self.read_u8(addr);
                let result = (self.reg_a as i32) - (value as i32);
                self.update_flags_zcn(result);
                self.cycles(4);
            }
            0xd9 => {
                //CMP,ABSY,3,4,CZidbv
                opcode_name = String::from("CMP");
                let offset = self.reg_y;
                let addr = self.get_absolute_addr(offset);
                let value = self.read_u8(addr);
                let result = (self.reg_a as i32) - (value as i32);
                self.update_flags_zcn(result);
                self.cycles(4);
            }
            0xc1 => {
                //CMP,INDX,2,6,CZidbv
                opcode_name = String::from("CMP");
                let addr = self.get_indirect_x_addr();
                let value = self.read_u8(addr);
                let result = (self.reg_a as i32) - (value as i32);
                self.update_flags_zcn(result);
                self.cycles(6);
            }
            0xd1 => {
                //CMP,INDY,2,5,CZidbv
                opcode_name = String::from("CMP");
                let addr = self.get_indirect_y_addr();
                let value = self.read_u8(addr);
                let result = (self.reg_a as i32) - (value as i32);
                self.update_flags_zcn(result);
                self.cycles(5);
            }
            0xe0 => {
                //CPX,IMM,2,2,CZidbVN
                opcode_name = String::from("CPX");
                let value = self.read_pc();
                let result = (self.reg_x as i32) - (value as i32);
                self.update_flags_zcn(result);
                self.cycles(2);
            }
            0xe4 => {
                //CPX,ZP,2,3,CZidbVN
                opcode_name = String::from("CPX");
                let addr = self.get_zeropage_addr(0);
                let value = self.read_u8(addr);
                let result = (self.reg_x as i32) - (value as i32);
                self.update_flags_zcn(result);
                self.cycles(3);
            }
            0xec => {
                //CPX,ABS,3,4,CZidbVN
                opcode_name = String::from("CPX");
                let addr = self.get_absolute_addr(0);
                let value = self.read_u8(addr);
                let result = (self.reg_x as i32) - (value as i32);
                self.update_flags_zcn(result);
                self.cycles(4);
            }
            0xc0 => {
                //CPY,IMM,2,2,CZidbVN
                opcode_name = String::from("CPY");
                let value = self.read_pc();
                let result = (self.reg_y as i32) - (value as i32);
                self.update_flags_zcn(result);
                self.cycles(2);
            }
            0xc4 => {
                //CPY,ZP,2,3,CZidbVN
                opcode_name = String::from("CPY");
                let addr = self.get_zeropage_addr(0);
                let value = self.read_u8(addr);
                let result = (self.reg_y as i32) - (value as i32);
                self.update_flags_zcn(result);
                self.cycles(3);
            }
            0xcc => {
                //CPY,ABS,3,4,
                opcode_name = String::from("CPY");
                let addr = self.get_absolute_addr(0);
                let value = self.read_u8(addr);
                let result = (self.reg_y as i32) - (value as i32);
                self.update_flags_zcn(result);
                self.cycles(4);
            }
            0xc6 => {
                //DEC,ZP,2,5,cZidbVN
                opcode_name = String::from("DEC");
                let address = self.get_zeropage_addr(0);
                let mut value = self.read_u8(address);
                value = value.wrapping_sub(1);
                self.write_u8(address, value);
                self.cycles(5);
            }
            0xd6 => {
                //DEC,ZPX,2,6,cZidbVN
                opcode_name = String::from("DEC");
                let offset = self.reg_x;
                let address = self.get_zeropage_addr(offset);
                let mut value = self.read_u8(address);
                value = value.wrapping_sub(1);
                self.write_u8(address, value);
                self.cycles(6);
            }
            0xce => {
                //DEC,ABS,3,6,cZidbVN
                opcode_name = String::from("DEC");
                let address = self.get_absolute_addr(0);
                let mut value = self.read_u8(address);
                value = value.wrapping_sub(1);
                self.write_u8(address, value);
                self.cycles(6);
            }
            0xde => {
                //DEC,ABSX,3,7,cZidbv
                opcode_name = String::from("DEC");
                let offset = self.reg_x;
                let address = self.get_absolute_addr(offset);
                let mut value = self.read_u8(address);
                value = value.wrapping_sub(1);
                self.write_u8(address, value);
                self.cycles(6);
            }
            0xca => {
                //DEX,IMP,1,2,cZidbVN
                opcode_name = String::from("DEX");
                if self.reg_x == 0 {
                    self.reg_x = 0xff
                } else {
                    self.reg_x = self.reg_x - 1;
                }
                let value = self.reg_x;
                self.update_flags_zn(value);
                self.cycles(2);
            }
            0x88 => {
                //DEY,IMP,1,2,cZidbVN
                opcode_name = String::from("DEY");
                if self.reg_y == 0 {
                    self.reg_y = 0xff
                } else {
                    self.reg_y = self.reg_y - 1;
                }
                let value = self.reg_y;
                self.update_flags_zn(value);
                self.cycles(2);
            }
            0xe8 => {
                //INX,IMP,1,2,cZidbVN
                opcode_name = String::from("INX");
//                self.reg_x = self.reg_x + 1;
                if self.reg_x == 0xff {
                    self.reg_x = 0;                        
                } else {
                    self.reg_x = self.reg_x + 1;                        
                }
                let value = self.reg_x;
                self.update_flags_zn(value);
                self.cycles(2);
            }
            0xc8 => {
                //INY,IMP,1,2,cZidbVN
                opcode_name = String::from("INY");
//                self.reg_y = self.reg_y + 1;
                if self.reg_y == 0xff {
                    self.reg_y = 0;                        
                } else {
                    self.reg_y = self.reg_y + 1;                        
                }
                let value = self.reg_y;
                self.update_flags_zn(value);
                self.cycles(2);
            }
            0x49 => {
                //EOR,IMM,2,2,cZidbVN
                opcode_name = String::from("EOR");
                let value = self.read_pc();
                let result = self.reg_a ^ value;
                self.reg_a = result;
                self.update_flags_zn(result);
                self.cycles(2);
            }
            0x45 => {
                //EOR,ZP,2,3,cZidbVN
                opcode_name = String::from("EOR");
                let addr = self.get_zeropage_addr(0);
                let value = self.read_u8(addr);
                let result = self.reg_a ^ value;
                self.reg_a = result;
                self.update_flags_zn(result);
                self.cycles(3);
            }
            0x55 => {
                //EOR,ZPX,2,4,cZidbVN
                opcode_name = String::from("EOR");
                let offset = self.reg_x;
                let addr = self.get_zeropage_addr(offset);
                let value = self.read_u8(addr);
                let result = self.reg_a ^ value;
                self.reg_a = result;
                self.update_flags_zn(result);
                self.cycles(4);
            }
            0x4d => {
                //EOR,ABS,3,4,cZidbVN
                opcode_name = String::from("EOR");
                let addr = self.get_absolute_addr(0);
                let value = self.read_u8(addr);
                let result = self.reg_a ^ value;
                self.reg_a = result;                        
                self.update_flags_zn(result);
                self.cycles(4);
            }
            0x5d => {
                //EOR,ABSX,3,4,cZidbv
                opcode_name = String::from("EOR");
                let offset = self.reg_x;
                let addr = self.get_absolute_addr(offset);
                let value = self.read_u8(addr);
                let result = self.reg_a ^ value;
                self.reg_a = result;                        
                self.update_flags_zn(result);
                self.cycles(4);
            }
            0x59 => {
                //EOR,ABSY,3,4,cZidbv
                opcode_name = String::from("EOR");
                let offset = self.reg_y;
                let addr = self.get_absolute_addr(offset);
                let value = self.read_u8(addr);
                let result = self.reg_a ^ value;
                self.reg_a = result;                        
                self.update_flags_zn(result);
                self.cycles(4);
            }
            0x41 => {
                //EOR,INDX,2,6,cZidbv
                opcode_name = String::from("EOR");
                let addr = self.get_indirect_x_addr();
                let value = self.read_u8(addr);
                let result = self.reg_a ^ value;
                self.reg_a = result;                        
                self.update_flags_zn(result);
                self.cycles(6);
            }
            0x51 => {
                //EOR,INDY,2,5,cZidbv
                opcode_name = String::from("EOR");
                let addr = self.get_indirect_y_addr();
                let value = self.read_u8(addr);
                let result = self.reg_a ^ value;
                self.reg_a = result;                        
                self.update_flags_zn(result);
                self.cycles(6);
            }
            0xe6 => {
                //INC,ZP,2,5,cZidbVN
                opcode_name = String::from("INC");
                let address = self.get_zeropage_addr(0);
                let mut value = self.read_u8(address);
                if value == 0xff {
                    value = 0;
                } else {
                    value = value + 1;
                }
                self.write_u8(address, value);
                self.cycles(5);
            }
            0xf6 => {
                //INC,ZPX,2,6,cZidbVN
                opcode_name = String::from("INC");
                let offset = self.reg_x;
                let address = self.get_zeropage_addr(offset);
                let mut value = self.read_u8(address);
                if value == 0xff {
                    value = 0;
                } else {
                    value = value + 1;
                }
                self.write_u8(address, value);
                self.cycles(6);
            }
            0xee => {
                //INC,ABS,3,6,cZidbVN
                opcode_name = String::from("INC");
                let address = self.get_absolute_addr(0);
                let mut value = self.read_u8(address);
                if value == 0xff {
                    value = 0;
                } else {
                    value = value + 1;
                }
                self.write_u8(address, value);
                self.cycles(6);
            }
            0xfe => {
                //INC,ABSX,3,7,cZidbv
                opcode_name = String::from("INC");
                let offset = self.reg_x;
                let address = self.get_absolute_addr(offset);
                let mut value = self.read_u8(address);
                if value == 0xff {
                    value = 0;
                } else {
                    value = value + 1;
                }
                self.write_u8(address, value);
                self.cycles(6);
            }
            0x4c => {
                //JMP,ABS,3,3,czidbVN
                opcode_name = String::from("JMP");
                let mut addr = self.read_pc() as u16;
                addr |= (self.read_pc() as u16) << 8;
                self.reg_pc = addr;
            }
            0x6c => {
                //JMP,IND,3,5,czidbVN
                opcode_name = String::from("JMP");
                let mut addr = self.read_pc() as u16;
                addr |= (self.read_pc() as u16) << 8;
                let dest = self.get_indirect_addr(addr);
                self.reg_pc = dest;
            }
            0x20 => {
                //JSR,ABS,3,6,czidbVN
                opcode_name = String::from("JSR");
                let mut addr = self.read_pc() as u16;
                addr |= (self.read_pc() as u16) << 8;
                let reg_pc = self.reg_pc;
                self.stack_push((reg_pc >> 8) as u8);
                self.stack_push(reg_pc as u8);
                self.reg_pc = addr;
                self.cycles(3);                        
            }
            0xa9 => {
                //LDA,IMM,2,2,cZidbVN
                opcode_name = String::from("LDA");
                let val = self.read_pc();
                self.reg_a = val;
                self.update_flags_zn(val);
                self.cycles(2);
            }
            0xa5 => {
                //LDA,ZP,2,3,cZidbVN
                opcode_name = String::from("LDA");
                let addr = self.get_zeropage_addr(0);
                let value = self.read_u8(addr);
                self.reg_a = value;
                self.update_flags_zn(value);
                self.cycles(3);
            }
            0xb5 => {
                //LDA,ZPX,2,4,cZidbVN
                opcode_name = String::from("LDA");
                let offset = self.reg_x;
                let addr = self.get_zeropage_addr(offset);
                let value = self.read_u8(addr);
                self.reg_a = value;
                self.update_flags_zn(value);
                self.cycles(3);
            }
            0xad => {
                //LDA,ABS,3,4,cZidbVN
                opcode_name = String::from("LDA");
                let addr = self.get_absolute_addr(0);
                let val = self.read_u8(addr);
                self.reg_a = val;
                self.update_flags_zn(val);
                self.cycles(4);
            }
            0xbd => {
                //LDA,ABSX,3,4,cZidbv
                opcode_name = String::from("LDA");
                let offset = self.reg_x;
                let addr = self.get_absolute_addr(offset);
                let val = self.read_u8(addr);
                self.reg_a = val;
                self.update_flags_zn(val);
                self.cycles(4);
            }
            0xb9 => {
                //LDA,ABSY,3,4,cZidbv
                opcode_name = String::from("LDA");
                let offset = self.reg_y;
                let addr = self.get_absolute_addr(offset);
                let val = self.read_u8(addr);
                self.reg_a = val;
                self.update_flags_zn(val);
                self.cycles(4);
            }
            0xa1 => {
                //LDA,INDX,2,6,cZidbv
                opcode_name = String::from("LDA");
                let addr = self.get_indirect_x_addr();
                let value = self.read_u8(addr);
                self.reg_a = value;
                self.update_flags_zn(value);
                self.cycles(6);
            }
            0xb1 => {
                //LDA,INDY,2,5,cZidbv
                opcode_name = String::from("LDA");
                let addr = self.get_indirect_y_addr();
                let value = self.read_u8(addr);
                self.reg_a = value;
                self.update_flags_zn(value);
                self.cycles(5);
            }
            0xa2 => {
                //LDX,IMM,2,2,cZidbVN
                opcode_name = String::from("LDX");
                let val = self.read_pc();
                self.reg_x = val;
                self.update_flags_zn(val);
                self.cycles(2);
            }
            0xa6 => {
                //LDX,ZP,2,3,cZidbVN
                opcode_name = String::from("LDX");
                let addr = self.get_zeropage_addr(0);
                let value = self.read_u8(addr);
                self.reg_x = value;
                self.update_flags_zn(value);
                self.cycles(3);
            }
            0xb6 => {
                //LDX,ZPY,2,4,cZidbVN
                opcode_name = String::from("LDX");
                let offset = self.reg_y;
                let addr = self.get_zeropage_addr(offset);
                let value = self.read_u8(addr);
                self.reg_x = value;
                self.update_flags_zn(value);
                self.cycles(4);
            }
            0xae => {
                //LDX,ABS,3,4,cZidbVN
                opcode_name = String::from("LDX");
                let addr = self.get_absolute_addr(0);
                let value = self.read_u8(addr);
                self.reg_x = value;
                self.update_flags_zn(value);
                self.cycles(4);
            }
            0xbe => {
                //LDX,ABSY,3,4,cZidbv
                opcode_name = String::from("LDX");
                let offset = self.reg_y;
                let addr = self.get_absolute_addr(offset);
                let value = self.read_u8(addr);
                self.reg_x = value;
                self.update_flags_zn(value);
                self.cycles(4);
            }
            0xa0 => {
                //LDY,IMM,2,2,cZidbVN
                opcode_name = String::from("LDY");
                let val = self.read_pc();
                self.reg_y = val;
                self.update_flags_zn(val);
                self.cycles(2);
            }
            0xa4 => {
                //LDY,ZP,2,3,cZidbVN
                opcode_name = String::from("LDY");
                let addr = self.get_zeropage_addr(0);
                let value = self.read_u8(addr);
                self.reg_y = value;
                self.update_flags_zn(value);
                self.cycles(3);
            }
            0xb4 => {
                //LDY,ZPX,2,4,cZidbVN
                opcode_name = String::from("LDY");
                let offset = self.reg_x;
                let addr = self.get_zeropage_addr(offset);
                let value = self.read_u8(addr);
                self.reg_y = value;
                self.update_flags_zn(value);
                self.cycles(4);
            }
            0xac => {
                //LDY,ABS,3,4,cZidbVN
                opcode_name = String::from("LDY");
                let addr = self.get_absolute_addr(0);
                let value = self.read_u8(addr);
                self.reg_y = value;
                self.update_flags_zn(value);
                self.cycles(4);
            }
            0xbc => {
                //LDY,ABSX,3,4,cZidbv
                opcode_name = String::from("LDY");
                let offset = self.reg_x;
                let addr = self.get_absolute_addr(offset);
                let value = self.read_u8(addr);
                self.reg_y = value;
                self.update_flags_zn(value);
                self.cycles(4);
            }
            0x4a => {
                //LSR,ACC,1,2,CZidbVN
                opcode_name = String::from("LSR");
                let value = self.reg_a;
                let result = self.lsr(value);
                self.reg_a = result;
                self.cycles(2);
            }
            0x46 => {
                //LSR,ZP,2,5,CZidbVN
                opcode_name = String::from("LSR");
                let addr = self.get_zeropage_addr(0);
                let value = self.read_u8(addr);
                let result = self.lsr(value);
                self.write_u8(addr, result);
                self.cycles(5);
            }
            0x56 => {
                //LSR,ZPX,2,6,CZidbVN
                opcode_name = String::from("LSR");
                let offset = self.reg_x;
                let addr = self.get_zeropage_addr(offset);
                let value = self.read_u8(addr);
                let result = self.lsr(value);
                self.write_u8(addr, result);
                self.cycles(5);
            }
            0x4e => {
                //LSR,ABS,3,6,CZidbVN
                opcode_name = String::from("LSR");
                let addr = self.get_absolute_addr(0);
                let value = self.read_u8(addr);
                let result = self.lsr(value);
                self.write_u8(addr, result);
                self.cycles(6);
            }
            0x5e => {
                //LSR,ABSX,3,7,CZidbv
                opcode_name = String::from("LSR");
                let offset = self.reg_x;
                let addr = self.get_absolute_addr(offset);
                let value = self.read_u8(addr);
                let result = self.lsr(value);
                self.write_u8(addr, result);
                self.cycles(7);
            }
            0x09 => {
                //ORA,IMM,2,2,cZidbVN
                opcode_name = String::from("ORA");
                let value = self.read_pc();
                let result = self.reg_a | value;
                self.reg_a = result;                        
                self.update_flags_zn(result);
                self.cycles(2);
            }
            0x05 => {
                //ORA,ZP,2,3,cZidbVN
                opcode_name = String::from("ORA");
                let addr = self.get_zeropage_addr(0);
                let value = self.read_u8(addr);
                let result = self.reg_a | value;
                self.reg_a = result;                        
                self.update_flags_zn(result);
                self.cycles(3);
            }
            0x15 => {
                //ORA,ZPX,2,4,cZidbVN
                opcode_name = String::from("ORA");
                let offset = self.reg_x;
                let addr = self.get_zeropage_addr(offset);
                let value = self.read_u8(addr);
                let result = self.reg_a | value;
                self.reg_a = result;
                self.update_flags_zn(result);
                self.cycles(4);
            }
            0x0d => {
                //ORA,ABS,3,4,cZidbVN
                opcode_name = String::from("ORA");
                let addr = self.get_absolute_addr(0);
                let value = self.read_u8(addr);
                let result = self.reg_a | value;
                self.reg_a = result;
                self.update_flags_zn(result);
                self.cycles(4);
            }
            0x1d => {
                //ORA,ABSX,3,4,cZidbv
                opcode_name = String::from("ORA");
                let offset = self.reg_x;
                let addr = self.get_absolute_addr(offset);
                let value = self.read_u8(addr);
                let result = self.reg_a | value;
                self.reg_a = result;
                self.update_flags_zn(result);
                self.cycles(4);
            }
            0x19 => {
                //ORA,ABSY,3,4,cZidbv
                opcode_name = String::from("ORA");
                let offset = self.reg_y;
                let addr = self.get_absolute_addr(offset);
                let value = self.read_u8(addr);
                let result = self.reg_a | value;
                self.reg_a = result;
                self.update_flags_zn(result);
                self.cycles(4);
            }
            0x01 => {
                //ORA,INDX,2,6,cZidbv
                opcode_name = String::from("ORA");
                let addr = self.get_indirect_x_addr();
                let value = self.read_u8(addr);
                let result = self.reg_a | value;
                self.reg_a = result;            
                self.update_flags_zn(result);
                self.cycles(6);
            }
            0x11 => {
                //ORA,INDY,2,5,cZidbv
                opcode_name = String::from("ORA");
                let addr = self.get_indirect_y_addr();
                let value = self.read_u8(addr);
                let result = self.reg_a | value;
                self.reg_a = result;
                self.update_flags_zn(result);
                self.cycles(5);
            }
            0x2a => {
                //ROL,ACC,1,2,CZidbVN
                opcode_name = String::from("ROL");
                let value = self.reg_a;
                let result = self.rol(value);
                self.reg_a = result;
                self.cycles(2);
            }
            0x26 => {
                //ROL,ZP,2,5,CZidbVN
                opcode_name = String::from("ROL");
                let addr = self.get_zeropage_addr(0);
                let value = self.read_u8(addr);
                let result = self.rol(value);
                self.write_u8(addr, result);
                self.cycles(5);
            }
            0x36 => {
                //ROL,ZPX,2,6,CZidbVN
                opcode_name = String::from("ROL");
                let offset = self.reg_x;
                let addr = self.get_zeropage_addr(offset);
                let value = self.read_u8(addr);
                let result = self.rol(value);
                self.write_u8(addr, result);
                self.cycles(6);
            }
            0x2e => {
                //ROL,ABS,3,6,
                opcode_name = String::from("ROL");
                let addr = self.get_absolute_addr(0);
                let value = self.read_u8(addr);
                let result = self.rol(value);
                self.write_u8(addr, result);
                self.cycles(6);
            }
            0x3e => {
                //ROL,ABSX,3,7,CZidbv
                opcode_name = String::from("ROL");
                let offset = self.reg_x;
                let addr = self.get_absolute_addr(offset);
                let value = self.read_u8(addr);
                let result = self.rol(value);
                self.write_u8(addr, result);                       
                self.cycles(6);
            }
            0x6a => {
                opcode_name = String::from("ROR");
                //ROR,ACC,1,2,CZidbVN
                let value = self.reg_a;
                let result = self.ror(value);
                self.reg_a = result;
                self.cycles(2);
            }
            0x66 => {
                //ROR,ZP,2,5,CZidbVN
                opcode_name = String::from("ROR");
                let addr = self.get_zeropage_addr(0);
                let value = self.read_u8(addr);
                let result = self.ror(value);
                self.write_u8(addr, result);                        
                self.cycles(5);
            }
            0x76 => {
                //ROR,ZPX,2,6,CZidbVN
                opcode_name = String::from("ROR");
                let offset = self.reg_x;
                let addr = self.get_zeropage_addr(offset);
                let value = self.read_u8(addr);
                let result = self.ror(value);
                self.write_u8(addr, result);                        
                self.cycles(6);
            }
            0x7e => {
                //ROR,ABSX,3,7,CZidbVN
                opcode_name = String::from("ROR");
                let offset = self.reg_x;
                let addr = self.get_absolute_addr(offset);
                let value = self.read_u8(addr);
                let result = self.ror(value);
                self.write_u8(addr, result);                        
                self.update_flags_zn(result);
                self.cycles(6);
            }
            0x6e => {
                //ROR,ABS,3,6,CZidbv
                let addr = self.get_absolute_addr(0);
                let value = self.read_u8(addr);
                let result = self.ror(value);
                self.write_u8(addr, result);                        
                self.update_flags_zn(result);
                self.cycles(6);
            }
            0xe9 => {
                //SBC,IMM,2,2,CZidbVN
                opcode_name = String::from("SBC");
                let value = self.read_pc();
                self.sbc(value);
                self.cycles(2);
            }
            0xe5 => {
                //SBC,ZP,2,3,CZidbVN
                opcode_name = String::from("SBC");
                let addr = self.get_zeropage_addr(0);
                let value = self.read_u8(addr);
                self.sbc(value);
                self.cycles(3);
            }
            0xf5 => {
                //SBC,ZPX,2,4,CZidbVN
                opcode_name = String::from("SBC");
                let offset = self.reg_x;
                let addr = self.get_zeropage_addr(offset);
                let value = self.read_u8(addr);
                self.sbc(value);
                self.cycles(4);
            }
            0xed => {
                //SBC,ABS,3,4,CZidbVN
                opcode_name = String::from("SBC");
                let addr = self.get_absolute_addr(0);
                let value = self.read_u8(addr);
                self.sbc(value);
                self.cycles(4);
            }
            0xfd => {
                //SBC,ABSX,3,4,CZidbv
                opcode_name = String::from("SBC");
                let offset = self.reg_x;
                let addr = self.get_absolute_addr(offset);
                let value = self.read_u8(addr);
                self.sbc(value);
                self.cycles(4);
            }
            0xf9 => {
                //SBC,ABSY,3,4,CZidbv
                opcode_name = String::from("SBC");
                let offset = self.reg_y;
                let addr = self.get_absolute_addr(offset);
                let value = self.read_u8(addr);
                self.sbc(value);
                self.cycles(4);
            }
            0xe1 => {
                //SBC,INDX,2,6,CZidbv
                opcode_name = String::from("SBC");
                let addr = self.get_indirect_x_addr();
                let value = self.read_u8(addr);
                self.sbc(value);
                self.cycles(6);
            }
            0xf1 => {
                //SBC,INDY,2,5,CZidbv
                opcode_name = String::from("SBC");
                let addr = self.get_indirect_y_addr();
                let value = self.read_u8(addr);
                self.sbc(value);
                self.cycles(5);
            }
            0x85 => {
                //STA,ZP,2,3,czidbVN
                opcode_name = String::from("STA");
                let addr = self.get_zeropage_addr(0);
                let value = self.reg_a;
                self.write_u8(addr, value);
                self.cycles(3);
            }
            0x95 => {
                //STA,ZPX,2,4,czidbVN
                opcode_name = String::from("STA");
                let offset = self.reg_x;
                let addr = self.get_zeropage_addr(offset);
                let value = self.reg_a;
                self.write_u8(addr, value);
                self.cycles(4);
            }
            0x8d => {
                //STA,ABS,3,4,czidbVN
                opcode_name = String::from("STA");
                let addr = self.get_absolute_addr(0);
                let val = self.reg_a;
                self.write_u8(addr, val);
                self.cycles(4);
            }
            0x9d => {
                //STA,ABSX,3,5,czidbv
                opcode_name = String::from("STA");
                let offset = self.reg_x;
                let addr = self.get_absolute_addr(offset);
                let val = self.reg_a;
                self.write_u8(addr, val);
                self.cycles(5);
            }
            0x99 => {
                //STA,ABSY,3,5,czidbv
                opcode_name = String::from("STA");
                let offset = self.reg_y;
                let addr = self.get_absolute_addr(offset);
                let val = self.reg_a;
                self.write_u8(addr, val);
                self.cycles(5);
            }
            0x81 => {
                //STA,INDX,2,6,czidbv
                opcode_name = String::from("STA");
                let offset = self.reg_x;
                let index = (offset as u16) + (self.read_pc() as u16);
                let addr = self.get_indirect_addr(index);
                let value = self.reg_a;
                self.write_u8(addr, value);
                self.cycles(6);
            }
            0x91 => {
                //STA,INDY,2,6,czidbv
                opcode_name = String::from("STA");
                let addr = self.get_indirect_y_addr();
                let value = self.reg_a;
                self.write_u8(addr, value);
                self.cycles(6);
            }
            0x86 => {
                //STX,ZP,2,3,czidbVN
                opcode_name = String::from("STX");
                let addr = self.get_zeropage_addr(0);
                let val = self.reg_x;
                self.write_u8(addr, val);
                self.cycles(3);
            }
            0x96 => {
                //STX,ZPY,2,4,czidbVN
                opcode_name = String::from("STX");
                let offset = self.reg_y;
                let addr = self.get_zeropage_addr(offset);
                let val = self.reg_x;
                self.write_u8(addr, val);
                self.cycles(4);
            }
            0x8e => {
                //STX,ABS,3,4,czidbVN
                opcode_name = String::from("STX");
                let addr = self.get_absolute_addr(0);
                let val = self.reg_x;
                self.write_u8(addr, val);
                self.cycles(4);
            }
            0x84 => {
                //STY,ZP,2,3,czidbVN
                opcode_name = String::from("STY");
                let addr = self.get_zeropage_addr(0);
                let val = self.reg_y;
                self.write_u8(addr, val);
                self.cycles(3);
            }
            0x94 => {
                //STY,ZPX,2,4,czidbVN
                opcode_name = String::from("STY");
                let offset = self.reg_x;
                let addr = self.get_zeropage_addr(offset);
                let val = self.reg_y;
                self.write_u8(addr, val);
                self.cycles(4);
            }
            0x8c => {
                //STY,ABS,3,4,czidbVN
                opcode_name = String::from("STY");
                let addr = self.get_absolute_addr(0);
                let val = self.reg_y;
                self.write_u8(addr, val);
                self.cycles(4);
            }
            _ => {

                for counter in &self.debug_vector {
                    println!("PC {:04x} OP {:02X} A {:02X} X {:02X} Y {:02X} R {:08b} {}", counter.pc, counter.op, counter.a, counter.x, counter.y, counter.registers, counter.opcode_name);
                }

                let registers = self.get_status_registers();
                println!("A {:02X}, X {:02X}, Y {:02X}, PC {:04X}, SP {:02X}, R {:08b}, OP {:02X}",
                    self.reg_a,
                    self.reg_x,
                    self.reg_y,
                    self.reg_pc,
                    self.reg_sp,
                    registers,
                    opcode
                );

                panic!("invalid opcopde {:02x} {:04x}", opcode, self.reg_pc);
            }
        }

        let r = self.get_status_registers();
        self.debug_vector.push_front(DebugFrame {
            pc : self.reg_pc,
            op : opcode, 
            a : self.reg_a,
            x : self.reg_x,
            y : self.reg_y,
            registers : r,
            opcode_name : opcode_name
        });

        if self.debug_vector.len() > 1000 {
            self.debug_vector.pop_back();
        }

        // if (starting_pc != 0xff2c ) && (starting_pc != 0xff29) {
        // #[cfg(debug_assertions)]
        // {
        //     println!("PC {:04X}, OP {:02X} {}, A {:02X}, X {:02X}, Y {:02X}, SP {:02X}, R {:08b}, 0x2b {:02X}",
        //         starting_pc,
        //         opcode,                
        //         opcode_name,
        //         self.reg_a,
        //         self.reg_x,
        //         self.reg_y,
        //         self.reg_sp,
        //         r,
        //         self.ram[0x2b]
        //     );
        // }
        // }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adc() {
        let mut cpu = MOS6502::new();

        cpu.reset();
        cpu.reg_a = 0x50;
        cpu.adc(0x10);
        assert_eq!(cpu.reg_a, 0x60);
        assert_eq!(cpu.f_negative, false);
        assert_eq!(cpu.f_zero, false);
        assert_eq!(cpu.f_carry, false);
        assert_eq!(cpu.f_overflow, false);

        cpu.reset();
        cpu.reg_a = 0x50;
        cpu.adc(0x50);
        assert_eq!(cpu.reg_a, 0xa0);
        assert_eq!(cpu.f_negative, true);
        assert_eq!(cpu.f_zero, false);
        assert_eq!(cpu.f_carry, false);
        assert_eq!(cpu.f_overflow, true);

        cpu.reset();
        cpu.reg_a = 0xd0;
        cpu.adc(0x90);
        assert_eq!(cpu.reg_a, 0x60);
        assert_eq!(cpu.f_negative, false);
        assert_eq!(cpu.f_zero, false);
        assert_eq!(cpu.f_carry, true);
        assert_eq!(cpu.f_overflow, true);

        cpu.reset();
        cpu.reg_a = 0x80;
        cpu.adc(0x80);
        assert_eq!(cpu.reg_a, 0x0);
        assert_eq!(cpu.f_negative, false);
        assert_eq!(cpu.f_zero, true);
        assert_eq!(cpu.f_carry, true);
        assert_eq!(cpu.f_overflow, true);
    }

    #[test]
    fn ror () {
        let mut cpu = MOS6502::new();
        cpu.reset();
        cpu.f_carry = true;
        let result = cpu.ror(108);
        assert_eq!(result, 182);
        assert_eq!(cpu.f_carry, false);
    }

    #[test]
    fn rol () {
        let mut cpu = MOS6502::new();
        cpu.reset();
        let result2 = cpu.rol(149);
        assert_eq!(result2 as u8, 42);
        assert_eq!(cpu.f_carry, true);
    }

    #[test]
    fn asl_true() {
        let value = 181;
        let result = (value as u16) << 1;
        let carry = result & 0xff00 > 0;
        let register = result as u8;
        assert_eq!(carry, true);
        assert_eq!(register, 106);
    }

    #[test]
    fn asl_false() {
        let value = 109;
        let result = (value as u16) << 1;
        let carry = result & 0xff00 > 0;
        let register = result as u8;
        assert_eq!(carry, false);
        assert_eq!(register, 218);
    }
}
