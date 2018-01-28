extern crate magpie;

use std::env;
use std::fs::File;

use std::io::prelude::*;
use std::thread;
use std::sync::mpsc;
use std::time::{Duration};

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

    cpu.load(buf, 0xC000);
    cpu.reset();

    let (tx, rx) = mpsc::channel();

    let handle = thread::spawn(move || {
        loop {

            let mut line = String::new();
            std::io::stdin().read_line(&mut line).expect("Failed to read line");
            let quit = line.as_str().starts_with("quit");
            tx.send(line).unwrap();

            if quit {
                break;
            }

            thread::sleep(Duration::from_millis(5));
        }
    });


    loop {

        let received = rx.try_recv();

        if !received.is_err() {

            let val = received.unwrap();

            if val.as_str().starts_with("C") {
                cpu.write_u8(0xf004, 0x43);
            } else {
                for v in val.as_str().bytes() {
                    if v >= 48 && v <= 90 && v == 0x0d {
                        cpu.write_u8(0xf004, v);
//                        println!("{:?}", v);
                    }
                }
            }
//            println!("try_recv {}", val);
            if val.as_str().starts_with("quit") {
                println!("quit main thread");
                break;
            }
        }

        if cpu.is_running() {
            cpu.step();
        } else {
            break;
        }

        //thread::sleep(Duration::from_millis(5));
    }

    let result = handle.join();

    println!("done {:?}", result.unwrap());


}