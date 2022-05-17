// runtime.rs

pub trait Runtime: Send {
    fn halted_get(&mut self) -> bool;
    fn halted_set(&mut self, halted: bool);

    fn flag_interrupt_get(&mut self) -> bool;
    fn flag_interrupt_set(&mut self, flag_interrupt: bool);

    fn raise(&mut self, vector: u16);

    fn step(&mut self);
}

impl Runtime for crate::Cpu {
    fn halted_get(&mut self) -> bool {
        self.halted
    }
    fn halted_set(&mut self, halted: bool) {
        self.halted = halted
    }

    fn flag_interrupt_get(&mut self) -> bool {
        self.flag.interrupt
    }
    fn flag_interrupt_set(&mut self, flag_interrupt: bool) {
        self.flag.interrupt = flag_interrupt
    }

    fn raise(&mut self, vector: u16) {
        self.interrupt(crate::Interrupt::Request(vector as u8));
    }

    fn step(&mut self) {
        self.execute_memory_instruction();
    }
}

impl Runtime for fox32core::State {
    fn halted_get(&mut self) -> bool {
        *self.halted()
    }
    fn halted_set(&mut self, halted: bool) {
        *self.halted() = halted;
    }

    fn flag_interrupt_get(&mut self) -> bool {
        *self.flag_interrupt()
    }
    fn flag_interrupt_set(&mut self, flag_interrupt: bool) {
        *self.flag_interrupt() = flag_interrupt;
    }

    fn raise(&mut self, vector: u16) {
        match fox32core::State::raise(self, vector) {
            Some(fox32core::Error::InterruptsDisabled) | None => {}
            Some(error) => {
                panic!("fox32core failed to raise interrupt {:#06X}: {}", vector, error);
            }
        }
    }

    fn step(&mut self) {
        if let Some(error) = fox32core::State::step(self) {
            panic!("fox32core failed to execute next instruction: {}", error);
        }
    }
}
