use std::io::{stdout, Write};
use platform::Platform;

pub const DSP : u16 = 0xf001;
pub const KBD : u16 = 0xf004;
const MEMORY_SIZE : usize = 65536;

pub struct Ehbasic {
    ram: [u8; MEMORY_SIZE]
}

impl Ehbasic {
    pub fn new() -> Ehbasic {
        Ehbasic {
            ram : [0; MEMORY_SIZE]
        }
    }
}

impl Platform for Ehbasic {

    fn read(&mut self, address: u16) -> u8 {
        let result = self.ram[address as usize];
        if address == KBD {
            self.ram[KBD as usize] = 0;
        }
        result
    }

    fn write(&mut self, address: u16, value: u8) {

        if address == DSP && value > 0 {
            print!("{}", value as char);
            stdout().flush().unwrap();
        }
        
        self.ram[address as usize] = value;
    }

    fn load(&mut self, program: Vec<u8>, address: u16) {
        self.ram = [0; MEMORY_SIZE];
        if program.len() > 0 {
            for i in 0..program.len() {
                let location = i + address as usize;
                self.ram[location] = program[i];
            }
        }
    }

    fn key_ready(&self) -> bool {
        self.ram[KBD as usize] == 0
    }

    fn key_pressed(&mut self, key: u8) {
        self.write(KBD, key);
    }

}
