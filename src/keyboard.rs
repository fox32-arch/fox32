// keyboard.rs

use crate::warn;

use ringbuf::{Consumer, Producer, RingBuffer};

pub struct Keyboard {
    consumer: Consumer<u32>,
    producer: Producer<u32>,
}

impl Keyboard {
    pub fn new() -> Self {
        let buffer = RingBuffer::<u32>::new(32);
        let (producer, consumer) = buffer.split();
        Keyboard { consumer, producer }
    }

    pub fn push(&mut self, scancode: u32) {
        self.producer.push(scancode).unwrap_or_else(|_| warn("keyboard buffer full!"));
    }

    pub fn pop(&mut self) -> u32 {
        self.consumer.pop().unwrap_or_default()
    }
}
