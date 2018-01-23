extern crate magpie;

use std::env;
use std::fs::File;
use std::io::prelude::*;

fn main() {

    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("missing argument(s)");
        return;
    }

    let filename = &args[1];
    //    let initial_pc = &args[2];

    println!("loading file {}", filename);

    let mut f = File::open(filename).expect("file not found");

    let mut buf: Vec<u8> = Vec::new();

    f.read_to_end(&mut buf).expect("error reading file");

    println!("loaded {} bytes", buf.len());

    let mut cpu = magpie::cpu::MOS6502::new();
    cpu.reset();

    cpu.load(buf, 0x600);

    let n = cpu.run(2048);

    println!("completed {} cycles", n);

}