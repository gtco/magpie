pub trait Platform {
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, value: u8);
    fn load(&mut self, program: Vec<u8>, address: u16);
    fn key_ready(&self) -> bool;
    fn key_pressed(&mut self, key: u8);
}
