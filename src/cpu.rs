pub const MEMORY_SIZE: usize = 64 * 1024;

pub struct MOS6502 {
    reg_a: u8,
    reg_x: u8,
    reg_y: u8,
    reg_sp: u8,
    reg_pc: u16,
    f_negative: bool,
    f_overflow: bool,
    f_break: bool,
    f_decimal: bool,
    f_interrupt: bool,
    f_zero: bool,
    f_carry: bool,

    cycle_count: i32,
    is_stopped: bool,

    ram: [u8; MEMORY_SIZE],
}

impl MOS6502 {
    pub fn new() -> MOS6502 {
        MOS6502 {
            reg_a: 0,
            reg_x: 0,
            reg_y: 0,
            reg_sp: 0xff,
            reg_pc: 0,
            f_negative: false,
            f_overflow: false,
            f_break: false,
            f_decimal: false,
            f_interrupt: false,
            f_zero: false,
            f_carry: false,
            cycle_count: 0,
            is_stopped: false,
            ram: [0; MEMORY_SIZE],
        }
    }

    pub fn reset(&mut self) {
            self.reg_a = 0;
            self.reg_x = 0;
            self.reg_y = 0;
            self.reg_sp = 0xff;
            self.reg_pc = 0;
            self.f_negative = false;
            self.f_overflow = false;
            self.f_break = false;
            self.f_decimal = false;
            self.f_interrupt = false;
            self.f_zero = false;
            self.f_carry = false;
    }

    pub fn load(&mut self, program: Vec<u8>, address: u16) {

        self.ram = [0; MEMORY_SIZE];

        for i in 0..program.len() {
            let location = i + address as usize;
            self.ram[location] = program[i];
        }

        self.reg_pc = address;
    }

    fn read_pc(&mut self) -> u8 {
        let addr = self.reg_pc;
        let ret = self.read_u8(addr);
        self.reg_pc += 1;
        // if self.reg_pc >= 0x45C0 {
        //     self.is_stopped = true;
        //     let result = self.read_u8(0x210);
        //     println!("result = {}", result);
        // }
        ret
    }

    fn read_u8(&mut self, address: u16) -> u8 {
        self.ram[address as usize]
    }

    fn write_u8(&mut self, address: u16, value: u8) {
        self.ram[address as usize] = value;
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
        println!("lo {:02x}, hi {:02x}", lo, hi);
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
        println!("stack push -> addr {:04x}, value {:02x} SP {:04x} ", addr, value, self.reg_sp);        
    }

    fn stack_pull(&mut self) -> u8 {
        self.reg_sp = self.reg_sp + 1;
        let addr = 0x100 + (self.reg_sp as u16) as u16;
        let result = self.read_u8(addr);

        println!("stack pull -> addr {:04x}, value {:02x}, SP {:02x}", addr, result, self.reg_sp);
        result
    }

    fn set_status_registers(&mut self, value: u8) {
        self.f_carry = (value & 0x01) == 0x01;
        self.f_zero = (value & 0x02) == 0x02;
        self.f_decimal = (value & 0x08) == 0x08;
        self.f_overflow = (value & 0x40) == 0x40;
        self.f_negative = (value & 0x80) == 0x80;
    }

    fn get_status_registers(&mut self) -> u8 {
        ((if self.f_negative { 1 } else { 0 }) << 7) |
        ((if self.f_overflow { 1 } else { 0 }) << 6) |
        ((if self.f_decimal { 1 } else { 0 }) << 3) |
        ((if self.f_zero { 1 } else { 0 }) << 1) |
        (if self.f_carry { 1 } else { 0 })
    }

    fn get_carry_amount(&mut self) -> u8 {
        if self.f_carry { 1 } else { 0 }
    }

    fn update_flags_zcn(&mut self, result: i32) {
        self.f_negative = (result & 0x80) == 0x80;
        self.f_zero = (result as u8) == 0;
        self.f_carry = result >= 0;
    }

    fn branch(&mut self) {
        let offset = self.read_pc() as u16;
        if (offset & 0x80) == 0x80 {
            self.reg_pc -= 0xff - offset;
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

    pub fn run(&mut self, target_cycles: i32) -> i32 {
        self.cycle_count = 0;
        let mut i = 0;
        while self.cycle_count < target_cycles {
            i = i + 1;
            if !self.is_stopped {
                // let result = self.read_u8(0x210);
                // println!("result = {}", result);

                let opcode = self.read_pc();
                let r = self.get_status_registers();
                println!(
                    "{:02}) A {:02X}, X {:02X}, Y {:02X}, PC {:04X}, SP {:02X}, R {:08b}, OP {:02X}",
                    i,
                    self.reg_a,
                    self.reg_x,
                    self.reg_y,
                    (self.reg_pc - 1),
                    self.reg_sp,
                    r, 
                    opcode
                );
                match opcode {
                    0x69 => {
                        //ADC,IMM,2,2,CZidbVN
                        let value = self.read_pc();
                        self.adc(value);
                        self.cycles(2);
                    }
                    0x65 => {
                        //ADC,ZP,2,3,CZidbVN
                        let addr = self.get_zeropage_addr(0);
                        let value = self.read_u8(addr);
                        self.adc(value);
                        self.cycles(3);
                    }
                    0x75 => {
                        //ADC,ZPX,2,4,CZidbVN
                        let offset = self.reg_x;
                        let addr = self.get_zeropage_addr(offset);
                        let value = self.read_u8(addr);
                        self.adc(value);
                        self.cycles(4);
                    }
                    0x6d => {
                        //ADC,ABS,3,4,CZidbVN
                        let addr = self.get_absolute_addr(0);
                        let value = self.read_u8(addr);
                        self.adc(value);
                        self.cycles(4);
                    }
                    0x7d => {
                        //ADC,ABSX,3,4,CZidbV
                        let offset = self.reg_x;
                        let addr = self.get_absolute_addr(offset);
                        let value = self.read_u8(addr);
                        self.adc(value);
                        self.cycles(4);
                    }
                    0x79 => {
                        //ADC,ABSY,3,4,CZidbV
                        let offset = self.reg_y;
                        let addr = self.get_absolute_addr(offset);
                        let value = self.read_u8(addr);
                        self.adc(value);
                        self.cycles(4);
                    }
                    0x61 => {
                        //ADC,INDX,2,6,CZidbV
                        let addr = self.get_indirect_x_addr();
                        let value = self.read_u8(addr);
                        self.adc(value);
                        self.cycles(6);
                    }
                    0x71 => {
                        //ADC,INDY,2,5,CZidbV
                        let addr = self.get_indirect_y_addr();
                        let value = self.read_u8(addr);
                        self.adc(value);
                        self.cycles(5);
                    }
                    0x29 => {
                        //AND,IMM,2,2,cZidbvN
                        let value = self.read_pc();
                        let result = self.reg_a & value;
                        self.reg_a = result;
                        self.update_flags_zn(result);
                        self.cycles(2);
                    }
                    0x25 => {
                        //AND,ZP,2,3,cZidbvN
                        let address = self.get_zeropage_addr(0);
                        let value = self.read_u8(address);
                        let result = self.reg_a & value;
                        self.reg_a = result;
                        self.update_flags_zn(result);
                        self.cycles(3);
                    }
                    0x35 => {
                        //AND,ZPX,2,4,cZidbvN
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
                        let address = self.get_absolute_addr(0);
                        let value = self.read_u8(address);
                        let result = self.reg_a & value;
                        self.reg_a = result;
                        self.update_flags_zn(result);
                        self.cycles(3);
                    }
                    0x3d => {
                        //AND,ABSX,3,4,cZidbv
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
                        let address = self.get_indirect_x_addr();
                        let value = self.read_u8(address);
                        let result = self.reg_a & value;
                        self.reg_a = result;
                        self.update_flags_zn(result);
                        self.cycles(6);
                    }
                    0x31 => {
                        //AND,INDY,2,5,cZidbv
                        let address = self.get_indirect_y_addr();
                        let value = self.read_u8(address);
                        let result = self.reg_a & value;
                        self.reg_a = result;
                        self.update_flags_zn(result);
                        self.cycles(5);
                    }
                    0x0a => {
                        //ASL,ACC,1,2,CZidbvN
                        let value = self.reg_a;
                        self.reg_a = self.asl(value);
                        self.cycles(2);
                    }
                    0x06 => {
                        //ASL,ZP,2,5,CZidbvN
                        let addr = self.get_zeropage_addr(0);
                        let mut value = self.read_u8(addr);
                        value = self.asl(value);
                        self.write_u8(addr, value);
                        self.cycles(5);
                    }
                    0x16 => {
                        //ASL,ZPX,2,6,CZidbvN
                        let offset = self.reg_x;
                        let addr = self.get_zeropage_addr(offset);
                        let mut value = self.read_u8(addr);
                        value = self.asl(value);
                        self.write_u8(addr, value);
                        self.cycles(5);
                    }
                    0x0e => {
                        //ASL,ABS,3,6,CZidbvN
                        let addr = self.get_absolute_addr(0);
                        let mut value = self.read_u8(addr);
                        value = self.asl(value);
                        self.write_u8(addr, value);
                        self.cycles(5);
                    }
                    0x1e => {
                        //ASL,ABSX,3,7,CZidbv
                        let offset = self.reg_x;
                        let addr = self.get_absolute_addr(offset);
                        let mut value = self.read_u8(addr);
                        value = self.asl(value);
                        self.write_u8(addr, value);
                        self.cycles(5);
                    }
                    0x90 => {
                        //BCC,REL,2,2/3,czidb
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
                        let addr = self.get_zeropage_addr(0);
                        let value = self.read_u8(addr);
                        let result = self.reg_a & value;
                        self.update_flags_zn(result);
                        self.cycles(3);
                    }
                    0x2c => {
                        //BIT,ABS,3,4,cZidbVN
                        let addr = self.get_absolute_addr(0);
                        let value = self.read_u8(addr);
                        let result = self.reg_a & value;
                        self.update_flags_zn(result);
                        self.cycles(4);
                    }
                    0x00 => {
                        //BRK,IMP,1,7,czidbVN
                        self.f_break = true;
                        self.cycles(2);
                        //TODO
                        self.is_stopped = true;
                    }
                    0x18 => {
                        //CLC,IMP,1,2,CzidbVN
                        self.f_carry = false;
                        self.cycles(2);
                    }
                    0xd8 => {
                        //CLD,IMP,1,2,czidbVN
                        self.f_decimal = false;
                        self.cycles(2);
                    }
                    0x58 => {
                        //CLI,IMP,1,2,czIdbVN
                        self.f_interrupt = false;
                        self.cycles(2);
                    }
                    0xb8 => {
                        //CLV,IMP,1,2,czidbVN
                        self.f_overflow = false;
                        self.cycles(2);
                    }
                    0xea => {
                        //NOP,IMP,1,2,czidbVN
                        self.nop()
                    }
                    0x48 => {
                        //PHA,IMP,1,3,czidbVN
                        let value = self.reg_a;
                        self.stack_push(value);
                        self.cycles(3);
                    }
                    0x68 => {
                        //PLA,IMP,1,4,cZidbVN
                        let value = self.stack_pull();
                        self.reg_a = value;
                        self.update_flags_zn(value);
                        self.cycles(4);
                    }
                    0x08 => {
                        //PHP,IMP,1,3,
                        let value = self.get_status_registers();
                        self.stack_push(value);
                        self.cycles(3);
                    }
                    0x28 => {
                        //PLP,IMP,1,4,CZIdbVN
                        let value = self.stack_pull();
                        self.set_status_registers(value);
                        self.cycles(4);
                    }
                    0x40 => {
                        //RTI,IMP,1,6,czidbVN
                        let psw = self.stack_pull();
                        self.set_status_registers(psw);
                        let mut addr = self.stack_pull() as u16;
                        addr |= (self.stack_pull() as u16) << 8;
                        self.cycles(6);
                        self.reg_pc = addr;
                    }
                    0x60 => {
                        //RTS,IMP,1,6,czidbVN
                        let mut addr = self.stack_pull() as u16;
                        addr |= (self.stack_pull() as u16) << 8;
                        let original = self.reg_pc;
                        self.reg_pc = addr;
                        println!("RTS initial {:04x}, addr {:04x}, destination {:04x}", original, addr, self.reg_pc);
                        self.cycles(2);                        
                    }
                    0x38 => {
                        //SEC,IMP,1,2,CzidbVN
                        self.f_carry = true;
                        self.cycles(2);
                    }
                    0xf8 => {
                        //SED,IMP,1,2,czidbVN
                        self.f_decimal = true;
                        self.cycles(2);
                    }
                    0x78 => {
                        //SEI,IMP,1,2,czIdbVN
                        self.f_interrupt = true;
                        self.cycles(2);
                    }
                    0xaa => {
                        //TAX,IMP,1,2,cZidbVN
                        let value = self.reg_a;
                        self.update_flags_zn(value);
                        self.reg_x = value;
                        self.cycles(2)
                    }
                    0x8a => {
                        //TXA,IMP,1,2,cZidbVN
                        let value = self.reg_x;
                        self.update_flags_zn(value);
                        self.reg_a = value;
                        self.cycles(2)
                    }
                    0xa8 => {
                        //TAY,IMP,1,2,cZidbVN
                        let value = self.reg_a;
                        self.update_flags_zn(value);
                        self.reg_y = value;
                        self.cycles(2)
                    }
                    0x98 => {
                        //TYA,IMP,1,2,cZidbVN
                        let value = self.reg_y;
                        self.update_flags_zn(value);
                        self.reg_a = value;
                        self.cycles(2)
                    }
                    0xba => {
                        //TSX,IMP,1,2,cZidbVN
                        let value = self.reg_sp;
                        self.update_flags_zn(value);
                        self.reg_x = value;
                    }
                    0x9a => {
                        //TXS,IMP,1,2,czidbVN
                        let value = self.reg_x;
                        self.update_flags_zn(value);
                        self.reg_sp = value;
                    }
                    0xc9 => {
                        //CMP,IMM,2,2,CZidbVN
                        let value = self.read_pc();
                        let result = (self.reg_a as i32) - (value as i32);
                        self.update_flags_zcn(result);
                        self.cycles(2);
                    }
                    0xc5 => {
                        //CMP,ZP,2,3,CZidbVN
                        let addr = self.get_zeropage_addr(0);
                        let value = self.read_u8(addr);
                        let result = (self.reg_a as i32) - (value as i32);
                        self.update_flags_zcn(result);
                        self.cycles(3);
                    }
                    0xd5 => {
                        //CMP,ZPX,2,4,CZidbVN
                        let offset = self.reg_x;
                        let addr = self.get_zeropage_addr(offset);
                        let value = self.read_u8(addr);
                        let result = (self.reg_a as i32) - (value as i32);
                        self.update_flags_zcn(result);
                        self.cycles(4);
                    }
                    0xcd => {
                        //CMP,ABS,3,4,CZidbVN
                        let addr = self.get_absolute_addr(0);
                        let value = self.read_u8(addr);
                        let result = (self.reg_a as i32) - (value as i32);
                        self.update_flags_zcn(result);
                        self.cycles(4);
                    }
                    0xdd => {
                        //CMP,ABSX,3,4,CZidbv
                        let offset = self.reg_x;
                        let addr = self.get_absolute_addr(offset);
                        let value = self.read_u8(addr);
                        let result = (self.reg_a as i32) - (value as i32);
                        self.update_flags_zcn(result);
                        self.cycles(4);
                    }
                    0xd9 => {
                        //CMP,ABSY,3,4,CZidbv
                        let offset = self.reg_y;
                        let addr = self.get_absolute_addr(offset);
                        let value = self.read_u8(addr);
                        let result = (self.reg_a as i32) - (value as i32);
                        self.update_flags_zcn(result);
                        self.cycles(4);
                    }
                    0xc1 => {
                        //CMP,INDX,2,6,CZidbv
                        let addr = self.get_indirect_x_addr();
                        let value = self.read_u8(addr);
                        let result = (self.reg_a as i32) - (value as i32);
                        self.update_flags_zcn(result);
                        self.cycles(6);
                    }
                    0xd1 => {
                        //CMP,INDY,2,5,CZidbv
                        let addr = self.get_indirect_y_addr();
                        let value = self.read_u8(addr);
                        let result = (self.reg_a as i32) - (value as i32);
                        self.update_flags_zcn(result);
                        self.cycles(5);
                    }
                    0xe0 => {
                        //CPX,IMM,2,2,CZidbVN
                        let value = self.read_pc();
                        let result = (self.reg_x as i32) - (value as i32);
                        self.update_flags_zcn(result);
                        self.cycles(2);
                    }
                    0xe4 => {
                        //CPX,ZP,2,3,CZidbVN
                        let addr = self.get_zeropage_addr(0);
                        let value = self.read_u8(addr);
                        let result = (self.reg_x as i32) - (value as i32);
                        self.update_flags_zcn(result);
                        self.cycles(3);
                    }
                    0xec => {
                        //CPX,ABS,3,4,CZidbVN
                        let addr = self.get_absolute_addr(0);
                        let value = self.read_u8(addr);
                        let result = (self.reg_x as i32) - (value as i32);
                        self.update_flags_zcn(result);
                        self.cycles(4);
                    }
                    0xc0 => {
                        //CPY,IMM,2,2,CZidbVN
                        let value = self.read_pc();
                        let result = (self.reg_y as i32) - (value as i32);
                        self.update_flags_zcn(result);
                        self.cycles(2);
                    }
                    0xc4 => {
                        //CPY,ZP,2,3,CZidbVN
                        let addr = self.get_zeropage_addr(0);
                        let value = self.read_u8(addr);
                        let result = (self.reg_y as i32) - (value as i32);
                        self.update_flags_zcn(result);
                        self.cycles(3);
                    }
                    0xcc => {
                        //CPY,ABS,3,4,
                        let addr = self.get_absolute_addr(0);
                        let value = self.read_u8(addr);
                        let result = (self.reg_y as i32) - (value as i32);
                        self.update_flags_zcn(result);
                        self.cycles(4);
                    }
                    0xc6 => {
                        //DEC,ZP,2,5,cZidbVN
                        let address = self.get_zeropage_addr(0);
                        let mut value = self.read_u8(address);
                        value = value.wrapping_sub(1);
                        self.write_u8(address, value);
                        self.cycles(5);
                    }
                    0xd6 => {
                        //DEC,ZPX,2,6,cZidbVN
                        let offset = self.reg_x;
                        let address = self.get_zeropage_addr(offset);
                        let mut value = self.read_u8(address);
                        value = value.wrapping_sub(1);
                        self.write_u8(address, value);
                        self.cycles(6);
                    }
                    0xce => {
                        //DEC,ABS,3,6,cZidbVN
                        let address = self.get_absolute_addr(0);
                        let mut value = self.read_u8(address);
                        value = value.wrapping_sub(1);
                        self.write_u8(address, value);
                        self.cycles(6);
                    }
                    0xde => {
                        //DEC,ABSX,3,7,cZidbv
                        let offset = self.reg_x;
                        let address = self.get_absolute_addr(offset);
                        let mut value = self.read_u8(address);
                        value = value.wrapping_sub(1);
                        self.write_u8(address, value);
                        self.cycles(6);
                    }
                    0xca => {
                        //DEX,IMP,1,2,cZidbVN
                        self.reg_x = self.reg_x - 1;
                        let value = self.reg_x;
                        self.update_flags_zn(value);
                        self.cycles(2);
                    }
                    0x88 => {
                        //DEY,IMP,1,2,cZidbVN
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
                        self.reg_x = self.reg_x + 1;
                        let value = self.reg_x;
                        self.update_flags_zn(value);
                        self.cycles(2);
                    }
                    0xc8 => {
                        //INY,IMP,1,2,cZidbVN
                        self.reg_y = self.reg_y + 1;
                        let value = self.reg_y;
                        self.update_flags_zn(value);
                        self.cycles(2);
                    }
                    0x49 => {
                        //EOR,IMM,2,2,cZidbVN
                        let value = self.read_pc();
                        let result = self.reg_a ^ value;
                        self.reg_a = result;
                        self.update_flags_zn(result);
                        self.cycles(2);
                    }
                    0x45 => {
                        //EOR,ZP,2,3,cZidbVN
                        let addr = self.get_zeropage_addr(0);
                        let value = self.read_u8(addr);
                        let result = self.reg_a ^ value;
                        self.reg_a = result;
                        self.update_flags_zn(result);
                        self.cycles(3);
                    }
                    0x55 => {
                        //EOR,ZPX,2,4,cZidbVN
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
                        let addr = self.get_absolute_addr(0);
                        let value = self.read_u8(addr);
                        let result = self.reg_a ^ value;
                        self.reg_a = result;                        
                        self.update_flags_zn(result);
                        self.cycles(4);
                    }
                    0x5d => {
                        //EOR,ABSX,3,4,cZidbv
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
                        let addr = self.get_indirect_x_addr();
                        let value = self.read_u8(addr);
                        let result = self.reg_a ^ value;
                        self.reg_a = result;                        
                        self.update_flags_zn(result);
                        self.cycles(6);
                    }
                    0x51 => {
                        //EOR,INDY,2,5,cZidbv
                        let addr = self.get_indirect_y_addr();
                        let value = self.read_u8(addr);
                        let result = self.reg_a ^ value;
                        self.reg_a = result;                        
                        self.update_flags_zn(result);
                        self.cycles(6);
                    }
                    0xe6 => {
                        //INC,ZP,2,5,cZidbVN
                        let address = self.get_zeropage_addr(0);
                        let mut value = self.read_u8(address);
                        value = value.wrapping_add(1);
                        self.write_u8(address, value);
                        self.cycles(5);
                    }
                    0xf6 => {
                        //INC,ZPX,2,6,cZidbVN
                        let offset = self.reg_x;
                        let address = self.get_zeropage_addr(offset);
                        let mut value = self.read_u8(address);
                        value = value.wrapping_add(1);
                        self.write_u8(address, value);
                        self.cycles(6);
                    }
                    0xee => {
                        //INC,ABS,3,6,cZidbVN
                        let address = self.get_absolute_addr(0);
                        let mut value = self.read_u8(address);
                        value = value.wrapping_add(1);
                        self.write_u8(address, value);
                        self.cycles(6);
                    }
                    0xfe => {
                        //INC,ABSX,3,7,cZidbv
                        let offset = self.reg_x;
                        let address = self.get_absolute_addr(offset);
                        let mut value = self.read_u8(address);
                        value = value.wrapping_add(1);
                        self.write_u8(address, value);
                        self.cycles(6);
                    }
                    0x4c => {
                        //JMP,ABS,3,3,czidbVN
                        let mut addr = self.read_pc() as u16;
                        addr |= (self.read_pc() as u16) << 8;
                        println!("JMP ABS initial {:04x}, destination {:04x}", self.reg_pc, addr);
                        self.reg_pc = addr;
                    }
                    0x6c => {
                        //JMP,IND,3,5,czidbVN
                        let mut addr = self.read_pc() as u16;
                        addr |= (self.read_pc() as u16) << 8;
                        let dest = self.get_indirect_addr(addr);
                        println!("JMP IND addr {:04x}, initial {:04x}, destination {:04x}", addr, self.reg_pc, dest);
                        self.reg_pc = dest;
                    }
                    0x20 => {
                        //JSR,ABS,3,6,czidbVN
                        let mut addr = self.read_pc() as u16;
                        addr |= (self.read_pc() as u16) << 8;
                        let reg_pc = self.reg_pc;
                        self.stack_push((reg_pc >> 8) as u8);
                        self.stack_push(reg_pc as u8);
                        self.reg_pc = addr;
                        println!("JSR initial {:04x}, destination {:04x}", reg_pc, addr);
                        self.cycles(3);                        
                    }
                    0xa9 => {
                        //LDA,IMM,2,2,cZidbVN
                        let val = self.read_pc();
                        self.reg_a = val;
                        self.update_flags_zn(val);
                        self.cycles(2);
                    }
                    0xa5 => {
                        //LDA,ZP,2,3,cZidbVN
                        let addr = self.get_zeropage_addr(0);
                        let value = self.read_u8(addr);
                        self.reg_a = value;
                        self.update_flags_zn(value);
                        self.cycles(3);
                    }
                    0xb5 => {
                        //LDA,ZPX,2,4,cZidbVN
                        let offset = self.reg_x;
                        let addr = self.get_zeropage_addr(offset);
                        let value = self.read_u8(addr);
                        self.reg_a = value;
                        self.update_flags_zn(value);
                        self.cycles(3);
                    }
                    0xad => {
                        //LDA,ABS,3,4,cZidbVN
                        let addr = self.get_absolute_addr(0);
                        let val = self.read_u8(addr);
                        self.reg_a = val;
                        self.update_flags_zn(val);
                        self.cycles(4);
                    }
                    0xbd => {
                        //LDA,ABSX,3,4,cZidbv
                        let offset = self.reg_x;
                        let addr = self.get_absolute_addr(offset);
                        let val = self.read_u8(addr);
                        self.reg_a = val;
                        self.update_flags_zn(val);
                        self.cycles(4);
                    }
                    0xb9 => {
                        //LDA,ABSY,3,4,cZidbv
                        let offset = self.reg_y;
                        let addr = self.get_absolute_addr(offset);
                        let val = self.read_u8(addr);
                        self.reg_a = val;
                        self.update_flags_zn(val);
                        self.cycles(4);
                    }
                    0xa1 => {
                        //LDA,INDX,2,6,cZidbv
                        let addr = self.get_indirect_x_addr();
                        let value = self.read_u8(addr);
                        self.reg_a = value;
                        self.update_flags_zn(value);
                        self.cycles(6);
                    }
                    0xb1 => {
                        //LDA,INDY,2,5,cZidbv
                        let addr = self.get_indirect_y_addr();
                        let value = self.read_u8(addr);
                        self.reg_a = value;
                        self.update_flags_zn(value);
                        self.cycles(5);
                    }
                    0xa2 => {
                        //LDX,IMM,2,2,cZidbVN
                        let val = self.read_pc();
                        self.reg_x = val;
                        self.update_flags_zn(val);
                        self.cycles(2);
                    }
                    0xa6 => {
                        //LDX,ZP,2,3,cZidbVN
                        let addr = self.get_zeropage_addr(0);
                        let value = self.read_u8(addr);
                        self.reg_x = value;
                        self.update_flags_zn(value);
                        self.cycles(3);
                    }
                    0xb6 => {
                        //LDX,ZPY,2,4,cZidbVN
                        let offset = self.reg_y;
                        let addr = self.get_zeropage_addr(offset);
                        let value = self.read_u8(addr);
                        self.reg_x = value;
                        self.update_flags_zn(value);
                        self.cycles(4);
                    }
                    0xae => {
                        //LDX,ABS,3,4,cZidbVN
                        let addr = self.get_absolute_addr(0);
                        let value = self.read_u8(addr);
                        self.reg_x = value;
                        self.update_flags_zn(value);
                        self.cycles(4);
                    }
                    0xbe => {
                        //LDX,ABSY,3,4,cZidbv
                        let offset = self.reg_y;
                        let addr = self.get_absolute_addr(offset);
                        let value = self.read_u8(addr);
                        self.reg_x = value;
                        self.update_flags_zn(value);
                        self.cycles(4);
                    }
                    0xa0 => {
                        //LDY,IMM,2,2,cZidbVN
                        let val = self.read_pc();
                        self.reg_y = val;
                        self.update_flags_zn(val);
                        self.cycles(2);
                    }
                    0xa4 => {
                        //LDY,ZP,2,3,cZidbVN
                        let addr = self.get_zeropage_addr(0);
                        let value = self.read_u8(addr);
                        self.reg_y = value;
                        self.update_flags_zn(value);
                        self.cycles(3);
                    }
                    0xb4 => {
                        //LDY,ZPX,2,4,cZidbVN
                        let offset = self.reg_x;
                        let addr = self.get_zeropage_addr(offset);
                        let value = self.read_u8(addr);
                        self.reg_y = value;
                        self.update_flags_zn(value);
                        self.cycles(4);
                    }
                    0xac => {
                        //LDY,ABS,3,4,cZidbVN
                        let addr = self.get_absolute_addr(0);
                        let value = self.read_u8(addr);
                        self.reg_y = value;
                        self.update_flags_zn(value);
                        self.cycles(4);
                    }
                    0xbc => {
                        //LDY,ABSX,3,4,cZidbv
                        let offset = self.reg_x;
                        let addr = self.get_absolute_addr(offset);
                        let value = self.read_u8(addr);
                        self.reg_y = value;
                        self.update_flags_zn(value);
                        self.cycles(4);
                    }
                    0x4a => {
                        //LSR,ACC,1,2,CZidbVN
                        let value = self.reg_a;
                        let result = self.lsr(value);
                        self.reg_a = result;
                        self.cycles(2);
                    }
                    0x46 => {
                        //LSR,ZP,2,5,CZidbVN
                        let addr = self.get_zeropage_addr(0);
                        let value = self.read_u8(addr);
                        let result = self.lsr(value);
                        self.write_u8(addr, result);
                        self.cycles(5);
                    }
                    0x56 => {
                        //LSR,ZPX,2,6,CZidbVN
                        let offset = self.reg_x;
                        let addr = self.get_zeropage_addr(offset);
                        let value = self.read_u8(addr);
                        let result = self.lsr(value);
                        self.write_u8(addr, result);
                        self.cycles(5);
                    }
                    0x4e => {
                        //LSR,ABS,3,6,CZidbVN
                        let addr = self.get_absolute_addr(0);
                        let value = self.read_u8(addr);
                        let result = self.lsr(value);
                        self.write_u8(addr, result);
                        self.cycles(6);
                    }
                    0x5e => {
                        //LSR,ABSX,3,7,CZidbv
                        let offset = self.reg_x;
                        let addr = self.get_absolute_addr(offset);
                        let value = self.read_u8(addr);
                        let result = self.lsr(value);
                        self.write_u8(addr, result);
                        self.cycles(7);
                    }
                    0x09 => {
                        //ORA,IMM,2,2,cZidbVN
                        let value = self.read_pc();
                        let result = self.reg_a | value;
                        self.reg_a = result;                        
                        self.update_flags_zn(result);
                        self.cycles(2);
                    }
                    0x05 => {
                        //ORA,ZP,2,3,cZidbVN
                        let addr = self.get_zeropage_addr(0);
                        let value = self.read_u8(addr);
                        let result = self.reg_a | value;
                        self.reg_a = result;                        
                        self.update_flags_zn(result);
                        self.cycles(3);
                    }
                    0x15 => {
                        //ORA,ZPX,2,4,cZidbVN
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
                        let addr = self.get_absolute_addr(0);
                        let value = self.read_u8(addr);
                        let result = self.reg_a | value;
                        self.reg_a = result;
                        self.update_flags_zn(result);
                        self.cycles(4);
                    }
                    0x1d => {
                        //ORA,ABSX,3,4,cZidbv
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
                        let addr = self.get_indirect_x_addr();
                        let value = self.read_u8(addr);
                        let result = self.reg_a | value;
                        self.reg_a = result;            
                        self.update_flags_zn(result);
                        self.cycles(6);
                    }
                    0x11 => {
                        //ORA,INDY,2,5,cZidbv
                        let addr = self.get_indirect_y_addr();
                        let value = self.read_u8(addr);
                        let result = self.reg_a | value;
                        self.reg_a = result;
                        self.update_flags_zn(result);
                        self.cycles(5);
                    }
                    0x2a => {
                        //ROL,ACC,1,2,CZidbVN
                        let value = self.reg_a;
                        let result = self.rol(value);
                        self.reg_a = result;
                        self.cycles(2);
                    }
                    0x26 => {
                        //ROL,ZP,2,5,CZidbVN
                        let addr = self.get_zeropage_addr(0);
                        let value = self.read_u8(addr);
                        let result = self.rol(value);
                        self.write_u8(addr, result);
                        self.cycles(5);
                    }
                    0x36 => {
                        //ROL,ZPX,2,6,CZidbVN
                        let offset = self.reg_x;
                        let addr = self.get_zeropage_addr(offset);
                        let value = self.read_u8(addr);
                        let result = self.rol(value);
                        self.write_u8(addr, result);
                        self.cycles(6);
                    }
                    0x2e => {
                        //ROL,ABS,3,6,CZidbVN
                        let addr = self.get_absolute_addr(0);
                        let value = self.read_u8(addr);
                        let result = self.rol(value);
                        println!("addr {:02x}, value {:02x}, result {:02x}", addr, value, result);
                        self.write_u8(addr, result);
                        self.cycles(6);
                    }
                    0x3e => {
                        //ROL,ABSX,3,7,CZidbv
                        let offset = self.reg_x;
                        let addr = self.get_absolute_addr(offset);
                        let value = self.read_u8(addr);
                        let result = self.rol(value);
                        self.write_u8(addr, result);                       
                        self.cycles(6);
                    }
                    0x6a => {
                        //ROR,ACC,1,2,CZidbVN
                        let value = self.reg_a;
                        let result = self.ror(value);
                        self.reg_a = result;
                        self.cycles(2);
                    }
                    0x66 => {
                        //ROR,ZP,2,5,CZidbVN
                        let addr = self.get_zeropage_addr(0);
                        let value = self.read_u8(addr);
                        let result = self.ror(value);
                        self.write_u8(addr, result);                        
                        self.cycles(5);
                    }
                    0x76 => {
                        //ROR,ZPX,2,6,CZidbVN
                        let offset = self.reg_x;
                        let addr = self.get_zeropage_addr(offset);
                        let value = self.read_u8(addr);
                        let result = self.ror(value);
                        self.write_u8(addr, result);                        
                        self.cycles(6);
                    }
                    0x7e => {
                        //ROR,ABSX,3,7,CZidbVN
                        let offset = self.reg_x;
                        let addr = self.get_absolute_addr(offset);
                        let value = self.read_u8(addr);
                        let result = self.ror(value);
                        println!("offset {:02x}, addr {:02x}, value {:02x}, result {:02x}", offset, addr, value, result);
                        self.write_u8(addr, result);                        
                        self.update_flags_zn(result);
                        self.cycles(6);
                    }
                    0x6e => {
                        //ROR,ABS,3,6,CZidbv
                        let addr = self.get_absolute_addr(0);
                        let value = self.read_u8(addr);
                        let result = self.ror(value);
                        println!("addr {:02x}, value {:02x}, result {:02x}", addr, value, result);
                        self.write_u8(addr, result);                        
                        self.update_flags_zn(result);
                        self.cycles(6);
                    }
                    0xe9 => {
                        //SBC,IMM,2,2,CZidbVN
                        let value = self.read_pc();
                        self.sbc(value);
                        self.cycles(2);
                    }
                    0xe5 => {
                        //SBC,ZP,2,3,CZidbVN
                        let addr = self.get_zeropage_addr(0);
                        let value = self.read_u8(addr);
                        self.sbc(value);
                        self.cycles(3);
                    }
                    0xf5 => {
                        //SBC,ZPX,2,4,CZidbVN
                        let offset = self.reg_x;
                        let addr = self.get_zeropage_addr(offset);
                        let value = self.read_u8(addr);
                        self.sbc(value);
                        self.cycles(4);
                    }
                    0xed => {
                        //SBC,ABS,3,4,CZidbVN
                        let addr = self.get_absolute_addr(0);
                        let value = self.read_u8(addr);
                        self.sbc(value);
                        self.cycles(4);
                    }
                    0xfd => {
                        //SBC,ABSX,3,4,CZidbv
                        let offset = self.reg_x;
                        let addr = self.get_absolute_addr(offset);
                        let value = self.read_u8(addr);
                        self.sbc(value);
                        self.cycles(4);
                    }
                    0xf9 => {
                        //SBC,ABSY,3,4,CZidbv
                        let offset = self.reg_y;
                        let addr = self.get_absolute_addr(offset);
                        let value = self.read_u8(addr);
                        self.sbc(value);
                        self.cycles(4);
                    }
                    0xe1 => {
                        //SBC,INDX,2,6,CZidbv
                        let addr = self.get_indirect_x_addr();
                        let value = self.read_u8(addr);
                        self.sbc(value);
                        self.cycles(6);
                    }
                    0xf1 => {
                        //SBC,INDY,2,5,CZidbv
                        let addr = self.get_indirect_y_addr();
                        let value = self.read_u8(addr);
                        self.sbc(value);
                        self.cycles(5);
                    }
                    0x85 => {
                        //STA,ZP,2,3,czidbVN
                        let addr = self.get_zeropage_addr(0);
                        let value = self.reg_a;
                        self.write_u8(addr, value);
                        self.cycles(3);
                    }
                    0x95 => {
                        //STA,ZPX,2,4,czidbVN
                        let offset = self.reg_x;
                        let addr = self.get_zeropage_addr(offset);
                        let value = self.reg_a;
                        self.write_u8(addr, value);
                        self.cycles(4);
                    }
                    0x8d => {
                        //STA,ABS,3,4,czidbVN
                        let addr = self.get_absolute_addr(0);
                        let val = self.reg_a;
                        self.write_u8(addr, val);
                        self.cycles(4);
                    }
                    0x9d => {
                        //STA,ABSX,3,5,czidbv
                        let offset = self.reg_x;
                        let addr = self.get_absolute_addr(offset);
                        let val = self.reg_a;
                        self.write_u8(addr, val);
                        self.cycles(5);
                    }
                    0x99 => {
                        //STA,ABSY,3,5,czidbv
                        let offset = self.reg_y;
                        let addr = self.get_absolute_addr(offset);
                        let val = self.reg_a;
                        self.write_u8(addr, val);
                        self.cycles(5);
                    }
                    0x81 => {
                        //STA,INDX,2,6,czidbv
                        let offset = self.reg_x;
                        let index = (offset as u16) + (self.read_pc() as u16);
                        let addr = self.get_indirect_addr(index);
                        let value = self.reg_a;
                        self.write_u8(addr, value);
                        self.cycles(6);
                    }
                    0x91 => {
                        //STA,INDY,2,6,czidbv
                        let offset = self.reg_y;
                        let index = self.read_pc() as u16;
                        let addr = self.get_indirect_addr(index) + (offset as u16);
                        let value = self.reg_a;
                        self.write_u8(addr, value);
                        self.cycles(6);
                    }
                    0x86 => {
                        //STX,ZP,2,3,czidbVN
                        let addr = self.get_zeropage_addr(0);
                        let val = self.reg_x;
                        self.write_u8(addr, val);
                        self.cycles(3);
                    }
                    0x96 => {
                        //STX,ZPY,2,4,czidbVN
                        let offset = self.reg_y;
                        let addr = self.get_zeropage_addr(offset);
                        let val = self.reg_x;
                        self.write_u8(addr, val);
                        self.cycles(4);
                    }
                    0x8e => {
                        //STX,ABS,3,4,czidbVN
                        let addr = self.get_absolute_addr(0);
                        let val = self.reg_x;
                        self.write_u8(addr, val);
                        self.cycles(4);
                    }
                    0x84 => {
                        //STY,ZP,2,3,czidbVN
                        let addr = self.get_zeropage_addr(0);
                        let val = self.reg_y;
                        self.write_u8(addr, val);
                        self.cycles(3);
                    }
                    0x94 => {
                        //STY,ZPX,2,4,czidbVN
                        let offset = self.reg_x;
                        let addr = self.get_zeropage_addr(offset);
                        let val = self.reg_y;
                        self.write_u8(addr, val);
                        self.cycles(4);
                    }
                    0x8c => {
                        //STY,ABS,3,4,czidbVN
                        let addr = self.get_absolute_addr(0);
                        let val = self.reg_y;
                        self.write_u8(addr, val);
                        self.cycles(4);
                    }

                    _ => {
                        println!(
                            "A {}, X {}, Y {}, PC {}",
                            self.reg_a,
                            self.reg_x,
                            self.reg_y,
                            self.reg_pc
                        );
                        panic!("invalid opcopde {}", opcode)
                    }
                }
            } else {
                self.cycles(2);
            }
        }
        self.cycle_count
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
