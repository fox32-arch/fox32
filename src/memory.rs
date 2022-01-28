// memory.rs

use crate::{Overlay};

use std::sync::{Arc, Mutex};

pub struct Memory {
    ram: Vec<u8>,
    rom: Vec<u8>,
    pub shared_memory: Arc<Mutex<Vec<u8>>>,
    pub overlays: Arc<Mutex<Vec<Overlay>>>,
}

impl Memory {
    pub fn new(size: u32, shared_memory: Arc<Mutex<Vec<u8>>>, overlays: Arc<Mutex<Vec<Overlay>>>, read_only_memory: Vec<u8>) -> Self {
        Memory {
            ram: vec![0; size as usize],
            rom: read_only_memory,
            shared_memory,
            overlays,
        }
    }
    pub fn read_8(&self, address: u32) -> u8 {
        let address = address as usize;
        if address < 0x02000000 {
            self.ram[address]
        } else if address >= 0xF0000000 {
            let address = address - 0xF0000000;
            self.rom[address]
        } else {
            let address = address - 0x02000000;
            let shared_memory_lock = self.shared_memory.lock().unwrap();
            shared_memory_lock[address]
        }
    }
    pub fn read_16(&self, address: u32) -> u16 {
        (self.read_8(address + 1) as u16) << 8 |
        (self.read_8(address) as u16)
    }
    pub fn read_32(&self, address: u32) -> u32 {
        let address = address as usize;
        if address < 0x02000000 {
            (self.ram[address + 3] as u32) << 24 |
            (self.ram[address + 2] as u32) << 16 |
            (self.ram[address + 1] as u32) << 8  |
            (self.ram[address] as u32)
        } else if address >= 0xF0000000 {
            let address = address - 0xF0000000;
            (self.rom[address + 3] as u32) << 24 |
            (self.rom[address + 2] as u32) << 16 |
            (self.rom[address + 1] as u32) << 8  |
            (self.rom[address] as u32)
        } else {
            let address = address - 0x02000000;
            let shared_memory_lock = self.shared_memory.lock().unwrap();
            (shared_memory_lock[address + 3] as u32) << 24 |
            (shared_memory_lock[address + 2] as u32) << 16 |
            (shared_memory_lock[address + 1] as u32) << 8  |
            (shared_memory_lock[address] as u32)
        }
    }
    pub fn write_8(&mut self, address: u32, byte: u8) {
        let address = address as usize;
        if address < 0x02000000 {
            self.ram[address] = byte;
        } else {
            let address = address - 0x02000000;
            let mut shared_memory_lock = self.shared_memory.lock().unwrap();
            shared_memory_lock[address] = byte;
        }
    }
    pub fn write_16(&mut self, address: u32, half: u16) {
        self.write_8(address, (half & 0x00FF) as u8);
        self.write_8(address + 1, (half >> 8) as u8);
    }
    pub fn write_32(&mut self, address: u32, word: u32) {
        let address = address as usize;
        if address < 0x02000000 {
            self.ram[address] = (word & 0x000000FF) as u8;
            self.ram[address + 1] = ((word & 0x0000FF00) >> 8) as u8;
            self.ram[address + 2] = ((word & 0x00FF0000) >> 16) as u8;
            self.ram[address + 3] = ((word & 0xFF000000) >> 24) as u8;
        } else {
            let address = address - 0x02000000;
            let mut shared_memory_lock = self.shared_memory.lock().unwrap();
            shared_memory_lock[address] = (word & 0x000000FF) as u8;
            shared_memory_lock[address + 1] = ((word & 0x0000FF00) >> 8) as u8;
            shared_memory_lock[address + 2] = ((word & 0x00FF0000) >> 16) as u8;
            shared_memory_lock[address + 3] = ((word & 0xFF000000) >> 24) as u8;
        }
    }
}