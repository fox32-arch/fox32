// wrapped.rs

use std::ops::Deref;
use std::cell::RefCell;
use std::cell::UnsafeCell;
use std::sync::Arc;

use crate::bus::Bus;
use crate::memory::*;
use crate::runtime::*;

#[derive(Clone)]
pub struct MemoryWrapped(Arc<dyn Memory>);

unsafe impl Send for MemoryWrapped {}
unsafe impl Sync for MemoryWrapped {}

impl MemoryWrapped {
    pub fn new(memory: impl Memory + 'static) -> Self {
        Self(Arc::new(memory))
    }
}

impl Memory for MemoryWrapped {
    fn ram(&self) -> &mut MemoryRam { self.0.ram() }
    fn rom(&self) -> &mut MemoryRom { self.0.rom() }
}

impl Memory for fox32core::State {
    fn ram(&self) -> &mut MemoryRam { self.memory_ram() }
    fn rom(&self) -> &mut MemoryRom { self.memory_rom() }
}

#[derive(Clone)]
pub struct BusWrapped(Arc<RefCell<Bus>>);

impl BusWrapped {
    pub fn new(bus: Bus) -> Self {
        Self(Arc::new(RefCell::new(bus)))
    }
}

impl Deref for BusWrapped {
    type Target = RefCell<Bus>;
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl fox32core::Bus for BusWrapped {
    fn io_read(&mut self, port: u32) -> Option<u32> {
        self.borrow_mut().io_read(port)
    }
    fn io_write(&mut self, port: u32, value: u32) -> Option<()> {
        self.borrow_mut().io_write(port, value)
    }
}

#[derive(Clone)]
pub struct CoreWrapped(Arc<UnsafeCell<fox32core::State>>);

unsafe impl Send for CoreWrapped {}

impl CoreWrapped {
    pub fn new(state: fox32core::State) -> Self {
        Self(Arc::new(UnsafeCell::new(state)))
    }

    fn inner(&self) -> *mut fox32core::State {
        self.0.get()
    }
}

impl Memory for CoreWrapped {
    fn ram(&self) -> &mut MemoryRam { unsafe { (*self.inner()).ram() } }
    fn rom(&self) -> &mut MemoryRom { unsafe { (*self.inner()).rom() } }
}

impl Runtime for CoreWrapped {
    fn halted_get(&mut self) -> bool {
        unsafe { (*self.inner()).halted_get() }
    }
    fn halted_set(&mut self, halted: bool) {
        unsafe { (*self.inner()).halted_set(halted) }
    }

    fn interrupts_enabled_get(&mut self) -> bool {
        unsafe { (*self.inner()).interrupts_enabled_get() }
    }
    fn interrupts_enabled_set(&mut self, interrupts_enabled: bool) {
        unsafe { (*self.inner()).interrupts_enabled_set(interrupts_enabled) }
    }

    fn raise(&mut self, vector: u16) {
        unsafe { Runtime::raise(&mut *self.inner(), vector) }
    }

    fn step(&mut self) {
        unsafe { Runtime::step(&mut *self.inner()) }
    }
}

#[derive(Clone)]
pub struct CoreMemoryWrapped(CoreWrapped);

unsafe impl Send for CoreMemoryWrapped {}
unsafe impl Sync for CoreMemoryWrapped {}

impl CoreMemoryWrapped {
    pub fn new(state_wrapped: CoreWrapped) -> Self {
        Self(state_wrapped)
    }
}

impl Memory for CoreMemoryWrapped {
    fn ram(&self) -> &mut MemoryRam { self.0.ram() }
    fn rom(&self) -> &mut MemoryRom { self.0.rom() }
}

