// memory.rs

use crate::warn;

use std::cell::UnsafeCell;
use std::sync::Arc;
use std::io::Write;

pub const MEMORY_RAM_SIZE: usize = 0x04000000; // 64 MiB
pub const MEMORY_ROM_SIZE: usize = 0x00080000; // 512 KiB

pub const MEMORY_RAM_START: usize = 0x00000000;
pub const MEMORY_ROM_START: usize = 0xF0000000;

pub type MemoryRam = [u8; MEMORY_RAM_SIZE];
pub type MemoryRom = [u8; MEMORY_ROM_SIZE];

struct MemoryInner {
    ram: Box<MemoryRam>,
    rom: Box<MemoryRom>,
}

impl MemoryInner {
    pub fn new(rom: &[u8]) -> Self {
        let mut this = Self {
            ram: Box::new([0u8; MEMORY_RAM_SIZE]),
            rom: Box::new([0u8; MEMORY_ROM_SIZE]),
        };
        this.rom.as_mut_slice().write(rom).expect("failed to copy ROM to memory");
        this
    }
}

#[derive(Clone)]
pub struct Memory(Arc<UnsafeCell<MemoryInner>>);

// SAFETY: once MemoryInner is initialzed, there is no way to modify the Box
//         pointers it contains and it does not matter if contents of the byte
//         arrays are corrupted
unsafe impl Send for Memory {}
unsafe impl Sync for Memory {}

impl Memory {
    pub fn new(rom: &[u8]) -> Self {
        Self(Arc::new(UnsafeCell::new(MemoryInner::new(rom))))
    }

    fn inner(&self) -> &mut MemoryInner {
        unsafe { &mut *self.0.get() }
    }

    pub fn ram(&self) -> &mut MemoryRam { &mut self.inner().ram }
    pub fn rom(&self) -> &mut MemoryRom { &mut self.inner().rom }

    pub fn read_8(&self, address: u32) -> u8 {
        let address = address as usize;

        let result = if address >= MEMORY_ROM_START && address < MEMORY_ROM_START + MEMORY_ROM_SIZE {
            self.rom().get(address - MEMORY_ROM_START)
        } else {
            self.ram().get(address - MEMORY_RAM_START)
        };

        match result {
            Some(value) => {
                *value
            }
            None => {
                warn(&format!("attempting to read from unmapped memory address: {:#010X}", address));
                0
            }
        }
    }
    pub fn read_16(&self, address: u32) -> u16 {
        (self.read_8(address) as u16) |
        (self.read_8(address + 1) as u16) << 8
    }
    pub fn read_32(&self, address: u32) -> u32 {
        (self.read_8(address) as u32) |
        (self.read_8(address + 1) as u32) <<  8 |
        (self.read_8(address + 2) as u32) << 16 |
        (self.read_8(address + 3) as u32) << 24
    }

    pub fn write_8(&self, address: u32, byte: u8) {
        let address = address as usize;

        if address >= MEMORY_ROM_START && address < MEMORY_ROM_START + MEMORY_ROM_SIZE {
            warn(&format!("attempting to write to ROM address: {:#010X}", address));
            return;
        }

        match self.ram().get_mut(address - MEMORY_RAM_START) {
            Some(value) => {
                *value = byte;
            }
            None => {
                warn(&format!("attempting to write to unmapped memory address: {:#010X}", address));
            }
        }
    }
    pub fn write_16(&self, address: u32, half: u16) {
        self.write_8(address, (half & 0x00FF) as u8);
        self.write_8(address + 1, (half >> 8) as u8);
    }
    pub fn write_32(&self, address: u32, word: u32) {
        self.write_8(address, (word & 0x000000FF) as u8);
        self.write_8(address + 1, ((word & 0x0000FF00) >>  8) as u8);
        self.write_8(address + 2, ((word & 0x00FF0000) >> 16) as u8);
        self.write_8(address + 3, ((word & 0xFF000000) >> 24) as u8);
    }
}