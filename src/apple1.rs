use std::io::{stdout, Write};
use platform::Platform;

const WOZMON: [u8; 256] = [
    0xd8, 0x58, 0xa0, 0x7f, 0x8c, 0x12, 0xd0, 0xa9, 0xa7, 0x8d, 0x11, 0xd0, 0x8d, 0x13, 0xd0, 0xc9,
    0xdf, 0xf0, 0x13, 0xc9, 0x9b, 0xf0, 0x03, 0xc8, 0x10, 0x0f, 0xa9, 0xdc, 0x20, 0xef, 0xff, 0xa9,
    0x8d, 0x20, 0xef, 0xff, 0xa0, 0x01, 0x88, 0x30, 0xf6, 0xad, 0x11, 0xd0, 0x10, 0xfb, 0xad, 0x10,
    0xd0, 0x99, 0x00, 0x02, 0x20, 0xef, 0xff, 0xc9, 0x8d, 0xd0, 0xd4, 0xa0, 0xff, 0xa9, 0x00, 0xaa,
    0x0a, 0x85, 0x2b, 0xc8, 0xb9, 0x00, 0x02, 0xc9, 0x8d, 0xf0, 0xd4, 0xc9, 0xae, 0x90, 0xf4, 0xf0,
    0xf0, 0xc9, 0xba, 0xf0, 0xeb, 0xc9, 0xd2, 0xf0, 0x3b, 0x86, 0x28, 0x86, 0x29, 0x84, 0x2a, 0xb9,
    0x00, 0x02, 0x49, 0xb0, 0xc9, 0x0a, 0x90, 0x06, 0x69, 0x88, 0xc9, 0xfa, 0x90, 0x11, 0x0a, 0x0a,
    0x0a, 0x0a, 0xa2, 0x04, 0x0a, 0x26, 0x28, 0x26, 0x29, 0xca, 0xd0, 0xf8, 0xc8, 0xd0, 0xe0, 0xc4,
    0x2a, 0xf0, 0x97, 0x24, 0x2b, 0x50, 0x10, 0xa5, 0x28, 0x81, 0x26, 0xe6, 0x26, 0xd0, 0xb5, 0xe6,
    0x27, 0x4c, 0x44, 0xff, 0x6c, 0x24, 0x00, 0x30, 0x2b, 0xa2, 0x02, 0xb5, 0x27, 0x95, 0x25, 0x95,
    0x23, 0xca, 0xd0, 0xf7, 0xd0, 0x14, 0xa9, 0x8d, 0x20, 0xef, 0xff, 0xa5, 0x25, 0x20, 0xdc, 0xff,
    0xa5, 0x24, 0x20, 0xdc, 0xff, 0xa9, 0xba, 0x20, 0xef, 0xff, 0xa9, 0xa0, 0x20, 0xef, 0xff, 0xa1,
    0x24, 0x20, 0xdc, 0xff, 0x86, 0x2b, 0xa5, 0x24, 0xc5, 0x28, 0xa5, 0x25, 0xe5, 0x29, 0xb0, 0xc1,
    0xe6, 0x24, 0xd0, 0x02, 0xe6, 0x25, 0xa5, 0x24, 0x29, 0x07, 0x10, 0xc8, 0x48, 0x4a, 0x4a, 0x4a,
    0x4a, 0x20, 0xe5, 0xff, 0x68, 0x29, 0x0f, 0x09, 0xb0, 0xc9, 0xba, 0x90, 0x02, 0x69, 0x06, 0x2c,
    0x12, 0xd0, 0x30, 0xfb, 0x8d, 0x12, 0xd0, 0x60, 0x00, 0x00, 0x00, 0x0f, 0x00, 0xff, 0x00, 0x00
];

pub const KBD : u16 = 0xd010;
pub const KBDCR : u16 = 0xd011;
pub const DSP : u16 = 0xd012;
pub const DSPCR : u16 = 0xd013;
const MEMORY_SIZE : usize = 65536;

pub struct Apple1 {
    ram: [u8; MEMORY_SIZE]
}

impl Apple1 {
    pub fn new() -> Apple1 {
        Apple1 {
            ram : [0; MEMORY_SIZE]
        }
    }
}

impl Platform for Apple1 {

    fn read(&mut self, address: u16) -> u8 {
        let result = self.ram[address as usize];
        if address == KBD {
            let kbdcr = self.ram[KBDCR as usize] & 0x7f;
            self.ram[KBDCR as usize] = kbdcr;
        }
        result
    }

    fn write(&mut self, address: u16, value: u8) {
        if address == KBD {
            let value = self.ram[KBDCR as usize] | 0x80;
            self.ram[KBDCR as usize] = value;
        }
        if address == DSP {
            if value > 0 {
                let ch = (value & 0x7f) as u8;
                if (ch == 0x0a) || (ch == 0x0d) {
                    println!();
                } else if (value & 0x7f) != 0x7f {
                    print!("{}",ch as char);
                }
                stdout().flush().unwrap();
            }
            self.ram[address as usize] = value & 0x7f;
        } else {
            self.ram[address as usize] = value;
        }
    }

    fn load(&mut self, program: Vec<u8>, address: u16) {
        self.ram = [0; MEMORY_SIZE];
        if program.len() > 0 {
            for i in 0..program.len() {
                let location = i + address as usize;
                self.ram[location] = program[i];
            }
        }
        for j in 0..WOZMON.len() {
            let location = j + 0xff00 as usize;
            self.ram[location] = WOZMON[j];
        }
    }

    fn key_ready(&self) -> bool {
        (self.ram[KBDCR as usize] & 0x80) != 80
    }

    fn key_pressed(&mut self, key: u8) {
        if key != 0x0a {
            self.write(KBD, key | 0x80);
            self.write(KBDCR, 0x80)
        }
    }
}