// keyboard.rs


use crate::warn;

use ringbuf::{Consumer, Producer, RingBuffer};
use std::sync::mpsc::Sender;
use winit::event::{ElementState, VirtualKeyCode};

pub struct Keyboard {
    consumer: Consumer<u8>,
    producer: Producer<u8>,
    debug_toggle_sender: Sender<()>,
}

impl Keyboard {
    pub fn new(debug_toggle_sender: Sender<()>) -> Self {
        let buffer = RingBuffer::<u8>::new(32);
        let (producer, consumer) = buffer.split();
        Keyboard { consumer, producer, debug_toggle_sender }
    }

    fn keycode_to_scancode(&self, keycode: VirtualKeyCode) -> u8 {
        match keycode {
            VirtualKeyCode::Escape => 0x01,
            VirtualKeyCode::Key1 => 0x02,
            VirtualKeyCode::Key2 => 0x03,
            VirtualKeyCode::Key3 => 0x04,
            VirtualKeyCode::Key4 => 0x05,
            VirtualKeyCode::Key5 => 0x06,
            VirtualKeyCode::Key6 => 0x07,
            VirtualKeyCode::Key7 => 0x08,
            VirtualKeyCode::Key8 => 0x09,
            VirtualKeyCode::Asterisk => 0x09,
            VirtualKeyCode::Key9 => 0x0A,
            VirtualKeyCode::Key0 => 0x0B,
            VirtualKeyCode::Minus => 0x0C,
            VirtualKeyCode::Equals => 0x0D,
            VirtualKeyCode::Back => 0x0E,
            VirtualKeyCode::Tab => 0x0F,
            VirtualKeyCode::Q => 0x10,
            VirtualKeyCode::W => 0x11,
            VirtualKeyCode::E => 0x12,
            VirtualKeyCode::R => 0x13,
            VirtualKeyCode::T => 0x14,
            VirtualKeyCode::Y => 0x15,
            VirtualKeyCode::U => 0x16,
            VirtualKeyCode::I => 0x17,
            VirtualKeyCode::O => 0x18,
            VirtualKeyCode::P => 0x19,
            VirtualKeyCode::LBracket => 0x1A,
            VirtualKeyCode::RBracket => 0x1B,
            VirtualKeyCode::Return => 0x1C,
            VirtualKeyCode::LControl => 0x1D,
            VirtualKeyCode::A => 0x1E,
            VirtualKeyCode::S => 0x1F,
            VirtualKeyCode::D => 0x20,
            VirtualKeyCode::F => 0x21,
            VirtualKeyCode::G => 0x22,
            VirtualKeyCode::H => 0x23,
            VirtualKeyCode::J => 0x24,
            VirtualKeyCode::K => 0x25,
            VirtualKeyCode::L => 0x26,
            VirtualKeyCode::Semicolon => 0x27,
            VirtualKeyCode::Apostrophe => 0x28,
            VirtualKeyCode::Grave => 0x29,
            VirtualKeyCode::LShift => 0x2A,
            VirtualKeyCode::Backslash => 0x2B,
            VirtualKeyCode::Z => 0x2C,
            VirtualKeyCode::X => 0x2D,
            VirtualKeyCode::C => 0x2E,
            VirtualKeyCode::V => 0x2F,
            VirtualKeyCode::B => 0x30,
            VirtualKeyCode::N => 0x31,
            VirtualKeyCode::M => 0x32,
            VirtualKeyCode::Comma => 0x33,
            VirtualKeyCode::Period => 0x34,
            VirtualKeyCode::Slash => 0x35,
            VirtualKeyCode::RShift => 0x36,
            VirtualKeyCode::LAlt => 0x38,
            VirtualKeyCode::Space => 0x39,
            VirtualKeyCode::Capital => 0x3A,
            VirtualKeyCode::F1 => 0x3B,
            VirtualKeyCode::F2 => 0x3C,
            VirtualKeyCode::F3 => 0x3D,
            VirtualKeyCode::F4 => 0x3E,
            VirtualKeyCode::F5 => 0x3F,
            VirtualKeyCode::F6 => 0x40,
            VirtualKeyCode::F7 => 0x41,
            VirtualKeyCode::F8 => 0x42,
            VirtualKeyCode::F9 => 0x43,
            VirtualKeyCode::F10 => 0x44,
            VirtualKeyCode::F11 => 0x57,
            VirtualKeyCode::F12 => 0x58,
            VirtualKeyCode::Up => 0x67,
            VirtualKeyCode::Down => 0x6C,
            VirtualKeyCode::Left => 0x69,
            VirtualKeyCode::Right => 0x6A,
            _ => 0x00,
        }
    }

    pub fn push(&mut self, keycode: Option<VirtualKeyCode>, state: ElementState) {
        let mut scancode = if let Some(keycode) = keycode {
            self.keycode_to_scancode(keycode)
        } else {
            0x00
        };
        if state == ElementState::Released && scancode != 0x00 {
            scancode |= 0x80; // "break" scancode
        }
        if scancode == 0x57 {
            self.debug_toggle_sender.send(()).unwrap();
            return;
        }
        self.producer.push(scancode).unwrap_or_else(|_| warn("keyboard buffer full!"));
    }

    pub fn pop(&mut self) -> u8 {
        self.consumer.pop().unwrap_or_default()
    }
}
