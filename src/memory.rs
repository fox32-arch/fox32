// memory.rs

use crate::Overlay;

use std::sync::{Arc, Mutex};

pub struct Memory {
    pub fast_memory: Vec<u8>,
    fast_memory_size: usize,
    pub shared_memory: Arc<Mutex<Vec<u8>>>,
    shared_memory_size: usize,

    pub overlays: Arc<Mutex<Vec<Overlay>>>,

    pub rom: Vec<u8>,
    rom_size: usize,
}

impl Memory {
    pub fn new(fast_memory: Vec<u8>, shared_memory: Arc<Mutex<Vec<u8>>>, overlays: Arc<Mutex<Vec<Overlay>>>, rom: Vec<u8>) -> Self {
        // 3 extra bytes at the end to allow for reading the last byte of memory as an immediate pointer
        let shared_memory_size = { shared_memory.lock().unwrap().len() - 3 };
        Memory {
            fast_memory_size: fast_memory.len() - 3,
            fast_memory,
            shared_memory_size,
            shared_memory,
            overlays,
            rom_size: rom.len(),
            rom,
        }
    }
    pub fn read_8(&self, address: u32) -> u8 {
        let address = address as usize;

        let fast_bottom_address = 0x00000000;
        let fast_top_address = fast_bottom_address + self.fast_memory_size - 1;

        let shared_bottom_address = 0x80000000;
        let shared_top_address = shared_bottom_address + self.shared_memory_size - 1;

        let rom_bottom_address = 0xF0000000;
        let rom_top_address = rom_bottom_address + self.rom_size - 1;

        if address >= fast_bottom_address && address <= fast_top_address {
            self.fast_memory[address]
        } else if address >= shared_bottom_address && address <= shared_top_address {
            let shared_memory_lock = self.shared_memory.lock().unwrap();
            shared_memory_lock[address - shared_bottom_address]
        } else if address >= rom_bottom_address && address <= rom_top_address {
            self.rom[address - rom_bottom_address]
        } else {
            println!("Warning: attempting to read unmapped memory address: {:#010X}", address);
            0
        }
    }
    pub fn read_16(&self, address: u32) -> u16 {
        (self.read_8(address + 1) as u16) << 8 |
        (self.read_8(address) as u16)
    }
    pub fn read_32(&self, address: u32) -> u32 {
        (self.read_8(address + 3) as u32) << 24 |
        (self.read_8(address + 2) as u32) << 16 |
        (self.read_8(address + 1) as u32) << 8  |
        (self.read_8(address) as u32)
    }
    pub fn write_8(&mut self, address: u32, byte: u8) {
        let address = address as usize;

        let fast_bottom_address = 0x00000000;
        let fast_top_address = fast_bottom_address + self.fast_memory_size - 1;

        let shared_bottom_address = 0x80000000;
        let shared_top_address = shared_bottom_address + self.shared_memory_size - 1;

        let rom_bottom_address = 0xF0000000;
        let rom_top_address = rom_bottom_address + self.rom_size - 1;

        if address >= fast_bottom_address && address <= fast_top_address {
            self.fast_memory[address] = byte;
        } else if address >= shared_bottom_address && address <= shared_top_address {
            let mut shared_memory_lock = self.shared_memory.lock().unwrap();
            shared_memory_lock[address - shared_bottom_address] = byte;
        } else if address >= rom_bottom_address && address <= rom_top_address {
            println!("Warning: attempting to write to ROM address: {:#010X}", address);
        } else {
            println!("Warning: attempting to write to unmapped memory address: {:#010X}", address);
        }
    }
    pub fn write_16(&mut self, address: u32, half: u16) {
        self.write_8(address, (half & 0x00FF) as u8);
        self.write_8(address + 1, (half >> 8) as u8);
    }
    pub fn write_32(&mut self, address: u32, word: u32) {
        self.write_8(address, (word & 0x000000FF) as u8);
        self.write_8(address + 1, ((word & 0x0000FF00) >> 8) as u8);
        self.write_8(address + 2, ((word & 0x00FF0000) >> 16) as u8);
        self.write_8(address + 3, ((word & 0xFF000000) >> 24) as u8);
    }
}