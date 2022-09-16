// setjmp.rs

use std::os::raw::*;

mod internal {
    use std::os::raw::*;
    use std::cell::*;

    extern "C" {
        pub fn setjmp(env: *mut c_void) -> c_int;
        pub fn longjmp(env: *mut c_void, status: c_int) -> !;
    }

    pub type JumpBuf = [u8; 512];

    pub struct JumpEnv<T>{
        buffer: Box<RefCell<JumpBuf>>,
        value: Cell<Option<T>>,
    }

    impl<T> JumpEnv<T> {
        pub fn new() -> Self {
            Self {
                buffer: Box::new(RefCell::new([0u8; 512])),
                value: Cell::default(),
            }
        }

        pub fn buffer(&self) -> RefMut<JumpBuf> {
            self.buffer.borrow_mut()
        }

        pub fn value_set(&self, value: T) {
            self.value.set(Some(value));
        }

        pub fn value_take(&self) -> Option<T> {
            self.value.take()
        }
    }
}

pub use internal::JumpEnv;

pub unsafe fn setjmp<T>(env: &JumpEnv<T>) -> Option<T> {
    if internal::setjmp(env.buffer().as_mut_ptr() as *mut c_void) != 0 {
        env.value_take()
    } else {
        None
    }
}

pub unsafe fn longjmp<T>(env: &JumpEnv<T>, value: T) -> ! {
    env.value_set(value);
    internal::longjmp(env.buffer().as_mut_ptr() as *mut c_void, 1)
}
