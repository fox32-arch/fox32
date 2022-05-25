// bus.rs

use crate::{Memory, Audio, DiskController, Keyboard, Mouse, Overlay};

use std::sync::{Arc, Mutex};
use std::io::{Write, stdout};

pub struct Bus {
    pub memory: Box<dyn Memory>,

    pub disk_controller: DiskController,

    pub audio: Arc<Mutex<Audio>>,

    pub keyboard: Arc<Mutex<Keyboard>>,
    pub mouse: Arc<Mutex<Mouse>>,

    pub overlays: Arc<Mutex<Vec<Overlay>>>,
}

impl Bus {
    pub fn read_io(&mut self, port: u32) -> u32 {
        match port {
            0x80000000..=0x8000031F => { // overlay port
                let overlay_lock = self.overlays.lock().unwrap();
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
                        overlay_lock[overlay_number].framebuffer_pointer
                    }
                    0x03 => {
                        // we're reading the enable status of this overlay
                        overlay_lock[overlay_number].enabled as u32
                    }
                    _ => panic!("invalid overlay control port"),
                }
            }
            0x80000400..=0x80000401 => { // mouse port
                let setting = (port & 0x000000FF) as u8;
                match setting {
                    0x00 => {
                        // we're reading the button states
                        let mut byte: u8 = 0x00;
                        let mouse_lock = self.mouse.lock().expect("failed to lock the mouse mutex");
                        if mouse_lock.clicked {
                            byte |= 0b001;
                        }
                        if mouse_lock.released {
                            byte |= 0b010;
                        }
                        if mouse_lock.held {
                            byte |= 0b100;
                        } else {
                            byte &= !0b100;
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
            0x80000500 => { // keyboard port
                let mut keyboard_lock = self.keyboard.lock().expect("failed to lock the keyboard mutex");
                keyboard_lock.pop() as u32
            }
            0x80001000..=0x80002003 => { // disk controller port
                let id = port as u8;
                let operation = (port & 0x0000F000) >> 8;

                match operation {
                    0x10 => {
                        // we're reading the current insert state of the specified disk id
                        if id > 3 {
                            panic!("invalid disk ID");
                        }
                        match &self.disk_controller.disk[id as usize] {
                            Some(disk) => disk.size() as u32, // return size if this disk is inserted
                            None => 0, // return 0 if this disk is not inserted
                        }
                    }
                    0x20 => {
                        // we're getting the location of the memory sector buffer
                        self.disk_controller.buffer_pointer as u32
                    }
                    _ => panic!("invalid disk controller port"),
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
            0x80000000..=0x8000031F => { // overlay port
                let mut overlay_lock = self.overlays.lock().unwrap();
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
                        overlay_lock[overlay_number].framebuffer_pointer = word;
                    }
                    0x03 => {
                        // we're setting the enable status of this overlay
                        overlay_lock[overlay_number].enabled = word != 0;
                    }
                    _ => panic!("invalid overlay control port"),
                }
            }
            0x80000400..=0x80000401 => { // mouse port
                let setting = (port & 0x000000FF) as u8;
                match setting {
                    0x00 => {
                        // we're setting the button states
                        let mut mouse_lock = self.mouse.lock().expect("failed to lock the mouse mutex");
                        mouse_lock.clicked = word & (1 << 0) != 0;
                        mouse_lock.released = word & (1 << 1) != 0;
                        mouse_lock.held = word & (1 << 2) != 0;
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
            0x80000600 => { // audio port
                let mut audio_lock = self.audio.lock().unwrap();
                audio_lock.playing = word != 0;
                audio_lock.current_buffer_is_0 = true;
            }
            0x80001000..=0x80005003 => { // disk controller port
                let id = port as u8;
                let operation = (port & 0x0000F000) >> 8;

                match operation {
                    0x10 => {
                        // we're requesting a disk to be inserted to the specified disk id
                        if id > 3 {
                            panic!("invalid disk ID");
                        }
                        let file = self.disk_controller.select_file();
                        match file {
                            Some(file) => self.disk_controller.insert(file, id),
                            None => {},
                        };
                    }
                    0x20 => {
                        // we're setting the location of the memory sector buffer
                        self.disk_controller.buffer_pointer = word as usize;
                    }
                    0x30 => {
                        // we're reading the specified sector of the specified disk id into the memory sector buffer
                        if id > 3 {
                            panic!("invalid disk ID");
                        }
                        self.disk_controller.set_current_sector(id, word);
                        self.disk_controller.read_into_memory(id, self.memory.ram());
                    }
                    0x40 => {
                        // we're writing the specified sector to the specified disk id from the memory sector buffer
                        if id > 3 {
                            panic!("invalid disk ID");
                        }
                        self.disk_controller.set_current_sector(id, word);
                        self.disk_controller.write_from_memory(id, self.memory.ram());
                    }
                    0x50 => {
                        // we're requesting a disk to be removed from the specified disk id
                        if id > 3 {
                            panic!("invalid disk ID");
                        }
                        self.disk_controller.remove(id);
                    }
                    _ => panic!("invalid disk controller port"),
                }
            }
            _ => return,
        }
    }
}

impl fox32core::Bus for Bus {
    fn io_read(&mut self, port: u32) -> Option<u32> {
        Some(self.read_io(port))
    }

    fn io_write(&mut self, port: u32, value: u32) -> Option<()> {
        self.write_io(port, value);
        Some(())
    }
}
