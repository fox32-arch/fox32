// bus.rs

use crate::{Memory, Mouse};

use std::convert::TryInto;
use std::io::{stdout, Write};
use std::sync::{Arc, Mutex};
use std::thread;

use rodio::{OutputStream, buffer::SamplesBuffer, Sink};

pub struct Bus {
    pub memory: Memory,
    pub mouse: Arc<Mutex<Mouse>>,
}

impl Bus {
    pub fn read_io(&mut self, port: u32) -> u32 {
        match port {
            0x02000000..=0x0200031F => { // overlay port
                let overlay_lock = self.memory.overlays.lock().unwrap();
                let overlay_number = (port & 0x000000FF) as usize;
                let setting = (port & 0x0000FF00) >> 8;

                match setting {
                    0x00 => {
                        // we're reading the position of this overlay
                        let x = overlay_lock[overlay_number].x as u32;
                        let y = overlay_lock[overlay_number].y as u32;
                        (y << 16) | x
                    }
                    0x01 => {
                        // we're reading the size of this overlay
                        let width = overlay_lock[overlay_number].width as u32;
                        let height = overlay_lock[overlay_number].height as u32;
                        (height << 16) | width
                    }
                    0x02 => {
                        // we're reading the framebuffer pointer of this overlay
                        overlay_lock[overlay_number].framebuffer_pointer + 0x02000000
                    }
                    0x03 => {
                        // we're reading the enable status of this overlay
                        overlay_lock[overlay_number].enabled as u32
                    }
                    _ => panic!("invalid overlay control port"),
                }
            }
            0x02000400..=0x02000401 => { // mouse port
                let setting = (port & 0x000000FF) as u8;
                match setting {
                    0x00 => {
                        // we're reading the button states
                        let mut byte: u8 = 0x00;
                        let mut mouse_lock = self.mouse.lock().expect("failed to lock the mouse mutex");
                        if mouse_lock.click {
                            byte |= 0b01;
                            mouse_lock.click = false;
                        }
                        if mouse_lock.held {
                            byte |= 0b10;
                        } else {
                            byte &= !0b10;
                        }
                        byte as u32
                    }
                    0x01 => {
                        // we're reading the position
                        let mouse_lock = self.mouse.lock().expect("failed to lock the mouse mutex");
                        let x = mouse_lock.x as u32;
                        let y = mouse_lock.y as u32;
                        (y << 16) | x
                    }
                    _ => panic!("invalid mouse control port"),
                }
            }
            _ => 0,
        }
    }
    pub fn write_io(&mut self, port: u32, word: u32) {
        match port {
            0x00000000 => { // terminal output port
                print!("{}", word as u8 as char);
                stdout().flush().expect("could not flush stdout");
            }
            0x02000000..=0x0200031F => { // overlay port
                let mut overlay_lock = self.memory.overlays.lock().unwrap();
                let overlay_number = (port & 0x000000FF) as usize;
                let setting = (port & 0x0000FF00) >> 8;

                match setting {
                    0x00 => {
                        // we're setting the position of this overlay
                        let x = (word & 0x0000FFFF) as usize;
                        let y = ((word & 0xFFFF0000) >> 16) as usize;
                        overlay_lock[overlay_number].x = x;
                        overlay_lock[overlay_number].y = y;
                    }
                    0x01 => {
                        // we're setting the size of this overlay
                        let width = (word & 0x0000FFFF) as usize;
                        let height = ((word & 0xFFFF0000) >> 16) as usize;
                        overlay_lock[overlay_number].width = width;
                        overlay_lock[overlay_number].height = height;
                    }
                    0x02 => {
                        // we're setting the framebuffer pointer of this overlay
                        if word < 0x02000000 {
                            panic!("overlay framebuffer must be within shared memory");
                        }
                        overlay_lock[overlay_number].framebuffer_pointer = word - 0x02000000;
                    }
                    0x03 => {
                        // we're setting the enable status of this overlay
                        overlay_lock[overlay_number].enabled = word != 0;
                    }
                    _ => panic!("invalid overlay control port"),
                }
            }
            0x02000320 => { // audio port
                if word < 0x02000000 {
                    panic!("audio buffer must be within shared memory");
                }
                let address = word as usize - 0x02000000;
                let shared_memory_lock = self.memory.shared_memory.lock().unwrap();

                let length = u32::from_le_bytes(shared_memory_lock[address..address+4].try_into().unwrap()) as usize;
                let start_address = address + 4;
                let end_address = start_address + length;

                let audio_data: Vec<i16> = shared_memory_lock[start_address..end_address].to_vec().chunks_exact(2).map(|x| ((x[1] as i16) << 8) | x[0] as i16).collect();
                let builder = thread::Builder::new().name("audio".to_string());
                builder.spawn({
                    move || {
                        let buffer = SamplesBuffer::new(1, 44100, audio_data);
                        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
                        let sink = Sink::try_new(&stream_handle).unwrap();
                        sink.append(buffer);
                        sink.sleep_until_end();
                    }
                }).unwrap();
            }
            0x02000400..=0x02000401 => { // mouse port
                let setting = (port & 0x000000FF) as u8;
                match setting {
                    0x00 => {
                        // we're setting the button states
                        let mut mouse_lock = self.mouse.lock().expect("failed to lock the mouse mutex");
                        mouse_lock.click = word & (1 << 0) != 0;
                        mouse_lock.held = word & (1 << 1) != 0;
                    }
                    0x01 => {
                        // we're setting the position
                        let mut mouse_lock = self.mouse.lock().expect("failed to lock the mouse mutex");
                        let x = (word & 0x0000FFFF) as u16;
                        let y = ((word & 0xFFFF0000) >> 16) as u16;
                        mouse_lock.x = x;
                        mouse_lock.y = y;
                    }
                    _ => panic!("invalid mouse control port"),
                }
            }
            _ => return,
        }
    }
}