// memory.rs

use std::alloc;
use std::cell::UnsafeCell;
use std::sync::Arc;
use std::io::Write;
use std::fs::File;

pub const MEMORY_RAM_SIZE: usize = 0x04000000; // 64 MiB
pub const MEMORY_ROM_SIZE: usize = 0x00080000; // 512 KiB

pub const MEMORY_RAM_START: usize = 0x00000000;
pub const MEMORY_ROM_START: usize = 0xF0000000;

pub type MemoryRam = [u8; MEMORY_RAM_SIZE];
pub type MemoryRom = [u8; MEMORY_ROM_SIZE];

pub trait Memory: Send {
    fn ram(&self) -> &mut MemoryRam;
    fn rom(&self) -> &mut MemoryRom;
}

impl dyn Memory {
    fn arrayslice<T, const ARRAY_LEN: usize, const SLICE_LEN: usize>(array: &mut [T; ARRAY_LEN], offset: usize) -> &mut [T; SLICE_LEN] {
        assert!(ARRAY_LEN >= SLICE_LEN + offset, "attempted slice [{}..{}] of array with length {}", offset, offset + SLICE_LEN, ARRAY_LEN);
        unsafe { &mut *array.as_mut_ptr().offset(offset as isize).cast() }
    }

    fn find<const S: usize>(&self, address: u32) -> Option<&mut [u8; S]> {
        let head = address as usize;
        let tail = head.checked_add(S).expect("unexpected overflow computing tail address");
        if head >= MEMORY_RAM_START && tail - MEMORY_RAM_START <= MEMORY_RAM_SIZE {
            return Self::arrayslice(self.ram(), head - MEMORY_RAM_START).into()
        }
        if head >= MEMORY_ROM_START && tail - MEMORY_ROM_START <= MEMORY_ROM_SIZE {
            return Self::arrayslice(self.rom(), head - MEMORY_ROM_START).into()
        }
        return None
    }
    fn find_unwrapped<const S: usize>(&self, address: u32) -> &mut [u8; S] {
        self.find(address).unwrap_or_else(|| panic!("attempted to access unmapped memory at {:#010X}", address))
    }

    pub fn read_8(&self, address: u32) -> u8 { u8::from_le_bytes(*self.find_unwrapped(address)) }
    pub fn read_16(&self, address: u32) -> u16 { u16::from_le_bytes(*self.find_unwrapped(address)) }
    pub fn read_32(&self, address: u32) -> u32 { u32::from_le_bytes(*self.find_unwrapped(address)) }

    pub fn write_8(&self, address: u32, value: u8) { *self.find_unwrapped(address) = u8::to_le_bytes(value) }
    pub fn write_16(&self, address: u32, value: u16) { *self.find_unwrapped(address) = u16::to_le_bytes(value) }
    pub fn write_32(&self, address: u32, value: u32) { *self.find_unwrapped(address) = u32::to_le_bytes(value) }

    pub fn dump(&self) {
        File::create("memory.dump")
            .expect("failed to open memory dump file")
            .write_all(self.ram())
            .expect("failed to write memory dump file");
    }
}

struct MemoryBufferInner {
    ram: *mut MemoryRam,
    rom: *mut MemoryRom,
}

static MEMORY_RAM_LAYOUT: alloc::Layout = alloc::Layout::new::<MemoryRam>();
static MEMORY_ROM_LAYOUT: alloc::Layout = alloc::Layout::new::<MemoryRom>();

impl Default for MemoryBufferInner {
    fn default() -> Self {
        unsafe {
            let ram = alloc::alloc_zeroed(MEMORY_RAM_LAYOUT);
            if ram.is_null() { alloc::handle_alloc_error(MEMORY_RAM_LAYOUT) }

            let rom = alloc::alloc_zeroed(MEMORY_ROM_LAYOUT);
            if rom.is_null() { alloc::handle_alloc_error(MEMORY_ROM_LAYOUT) }

            Self { ram: ram.cast(), rom: rom.cast() }
        }
    }
}

impl Drop for MemoryBufferInner {
    fn drop(&mut self) {
        unsafe {
            alloc::dealloc(self.ram.cast(), MEMORY_RAM_LAYOUT);
            alloc::dealloc(self.rom.cast(), MEMORY_ROM_LAYOUT);
        }
    }
}

#[derive(Default, Clone)]
pub struct MemoryBuffer(Arc<UnsafeCell<MemoryBufferInner>>);

unsafe impl Send for MemoryBuffer {}
unsafe impl Sync for MemoryBuffer {}

impl MemoryBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    fn inner(&self) -> &mut MemoryBufferInner {
        unsafe { &mut *self.0.get() }
    }
}

impl Memory for MemoryBuffer {
    fn ram(&self) -> &mut MemoryRam { unsafe { &mut *self.inner().ram } }
    fn rom(&self) -> &mut MemoryRom { unsafe { &mut *self.inner().rom } }
}

#[derive(Default, Clone, Copy)]
pub struct MemoryStub();

impl Memory for MemoryStub {
    fn ram(&self) -> &mut MemoryRam { unimplemented!() }
    fn rom(&self) -> &mut MemoryRom { unimplemented!() }
}
