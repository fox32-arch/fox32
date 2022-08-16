// memory.rs

use crate::error;
use crate::cpu::Exception;

use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::io::Write;
use std::fs::File;

pub const MEMORY_RAM_SIZE: usize = 0x04000000; // 64 MiB
pub const MEMORY_ROM_SIZE: usize = 0x00080000; // 512 KiB

pub const MEMORY_RAM_START: usize = 0x00000000;
pub const MEMORY_ROM_START: usize = 0xF0000000;

pub type MemoryRam = [u8; MEMORY_RAM_SIZE];
pub type MemoryRom = [u8; MEMORY_ROM_SIZE];

#[derive(Debug)]
pub struct MemoryPage {
    physical_address: u32,
    present: bool,
    rw: bool,
}

struct MemoryInner {
    ram: Box<MemoryRam>,
    rom: Box<MemoryRom>,
    mmu_enabled: Box<bool>,
    tlb: Box<HashMap<u32, MemoryPage>>,
    paging_directory_address: Box<u32>,
    exception_sender: Sender<Exception>,
}

impl MemoryInner {
    pub fn new(rom: &[u8], exception_sender: Sender<Exception>) -> Self {
        let mut this = Self {
            // HACK: allocate directly on the heap to avoid a stack overflow
            //       at runtime while trying to move around a 64MB array
            ram: unsafe { Box::from_raw(Box::into_raw(vec![0u8; MEMORY_RAM_SIZE].into_boxed_slice()) as *mut MemoryRam) },
            rom: unsafe { Box::from_raw(Box::into_raw(vec![0u8; MEMORY_ROM_SIZE].into_boxed_slice()) as *mut MemoryRom) },
            mmu_enabled: Box::from(false),
            tlb: Box::from(HashMap::with_capacity(1024)),
            paging_directory_address: Box::from(0x00000000),
            exception_sender,
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
    pub fn new(rom: &[u8], exception_sender: Sender<Exception>) -> Self {
        Self(Arc::new(UnsafeCell::new(MemoryInner::new(rom, exception_sender))))
    }

    fn inner(&self) -> &mut MemoryInner {
        unsafe { &mut *self.0.get() }
    }

    pub fn ram(&self) -> &mut MemoryRam { &mut self.inner().ram }
    pub fn rom(&self) -> &mut MemoryRom { &mut self.inner().rom }
    pub fn mmu_enabled(&self) -> &mut bool { &mut self.inner().mmu_enabled }
    pub fn tlb(&self) -> &mut HashMap<u32, MemoryPage> { &mut self.inner().tlb }
    pub fn paging_directory_address(&self) -> &mut u32 { &mut self.inner().paging_directory_address }
    pub fn exception_sender(&self) -> &mut Sender<Exception> { &mut self.inner().exception_sender }

    pub fn dump(&self) {
        let mut file = File::create("memory.dump").expect("failed to open memory dump file");
        file.write_all(self.ram()).expect("failed to write memory dump file");
    }

    pub fn flush_tlb(&self, paging_directory_address: Option<u32>) {
        let directory_address = if let Some(address) = paging_directory_address {
            *self.paging_directory_address() = address;
            address
        } else {
            *self.paging_directory_address()
        };

        self.tlb().clear();

        // each table contains 1024 entries
        // the paging directory contains pointers to paging tables with the following format:
        // bit 0: present
        // remaining bits are ignored, should be zero
        // bits 12-31: physical address of paging table

        // the paging table contains pointers to physical memory pages with the following format:
        // bit 0: present
        // bit 1: r/w
        // remaining bits are ignored, should be zero
        // bits 12-31: physical address

        for directory_index in 0..1024 {
            let directory = self.read_32(directory_address + (directory_index * 4));
            let dir_present = directory & 0b1 != 0;
            let dir_address = directory & 0xFFFFF000;
            if dir_present {
                for table_index in 0..1024 {
                    let table = self.read_32(dir_address + (table_index * 4));
                    let table_present = table & 0b01 != 0;
                    let table_rw = table & 0b10 != 0;
                    let table_address = table & 0xFFFFF000;

                    let tlb_entry = MemoryPage {
                        //physical_address: (((directory_index + 1) * (table_index + 1) * 4096) - 4096),
                        physical_address: (directory_index << 22) | (table_index << 12),
                        present: table_present,
                        rw: table_rw,
                    };
                    self.tlb().entry(table_address).or_insert(tlb_entry);
                }
            }
        }
        println!("{:#X?}", self.tlb());
    }

    pub fn virtual_to_physical(&self, virtual_address: u32) -> Option<(u32, bool)> {
        let virtual_page = virtual_address & 0xFFFFF000;
        let offset = virtual_address & 0x00000FFF;
        let physical_page = self.tlb().get(&virtual_page);
        let physical_address = match physical_page {
            Some(page) => {
                if page.present {
                    Some((page.physical_address | offset, page.rw))
                } else {
                    None
                }
            },
            None => None,
        };
        physical_address
    }

    pub fn read_8(&self, mut address: u32) -> u8 {
        if *self.mmu_enabled() {
            (address, _) = self.virtual_to_physical(address as u32).unwrap_or_else(|| {
                self.exception_sender().send(Exception::PageFault(address)).unwrap();
                (0, false)
            });
        }

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
                error(&format!("attempting to read from unmapped physical memory address: {:#010X}", address));
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

    pub fn write_8(&self, mut address: u32, byte: u8) {
        let mut writable = true;
        if *self.mmu_enabled() {
            (address, writable) = self.virtual_to_physical(address as u32).unwrap_or_else(|| {
                self.exception_sender().send(Exception::PageFault(address)).unwrap();
                (0, false)
            });
        }

        if writable {
            let address = address as usize;

            if address >= MEMORY_ROM_START && address < MEMORY_ROM_START + MEMORY_ROM_SIZE {
                error(&format!("attempting to write to ROM address: {:#010X}", address));
            }

            match self.ram().get_mut(address - MEMORY_RAM_START) {
                Some(value) => {
                    *value = byte;
                }
                None => {
                    error(&format!("attempting to write to unmapped physical memory address: {:#010X}", address));
                }
            }
        } else {
            self.exception_sender().send(Exception::PageFault(address)).unwrap();
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
