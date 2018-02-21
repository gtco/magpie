extern crate magpie;

use std::env;
use std::fs::File;

use std::io::prelude::*;
use std::thread;
use std::sync::mpsc;
use std::time::Duration;
use std::collections::VecDeque;

use magpie::platform::Platform;
use magpie::cpu::MOS6502;
use magpie::apple1::Apple1;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("missing argument(s)");
        return;
    }

    let buf = load_file(&args[1]);
    let mut apple1 = Apple1::new();
    apple1.load(buf, 0x4000);
    let mut cpu = MOS6502::new(Box::new(apple1));

    cpu.reset();
    cpu.run(1024);

    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || loop {
        let mut line = String::new();
        std::io::stdin()
            .read_line(&mut line)
            .expect("Failed to read line");
        let quit = line.as_str().starts_with("quit");
        tx.send(line).unwrap();
        if quit {
            break;
        }

        thread::sleep(Duration::from_millis(5));
    });

    let mut c: u64 = 0;
    let mut key_buffer: VecDeque<u8> = VecDeque::new();

    loop {
        if cpu.is_running() {
            let received = rx.try_recv();
            if !received.is_err() {
                let val = received.unwrap();
                for b in val.bytes() {
                    key_buffer.push_back(b);
                }
                if val.starts_with("quit") {
                    break;
                }
            }

            if !key_buffer.is_empty() && cpu.key_ready() {
                let v = key_buffer.pop_front().unwrap();
                cpu.key_pressed(v);
            }

            cpu.run(2 * 1024);

            thread::sleep(Duration::from_millis(100));
        } else {
            break;
        }

        c = c + 1;
    }

    let _result = handle.join();
    println!("done, iteration count = {:?}", c);
}

fn load_file(filename: &String) -> Vec<u8> {
    //let filename = &args[1];
    println!("loading file {}", filename);
    let mut f = File::open(filename).expect("file not found");
    let mut buf: Vec<u8> = Vec::new();
    f.read_to_end(&mut buf).expect("error reading file");
    println!("loaded {} bytes", buf.len());
    buf
}
