// bus.rs

use crate::{DiskController, Memory, Mouse};

use std::io::{stdout, Write};
use std::sync::{Arc, Mutex};

pub struct Bus {
    pub disk_controller: DiskController,
    pub memory: Memory,
    pub mouse: Arc<Mutex<Mouse>>,
}

impl Bus {
    pub fn read_io(&mut self, port: u32) -> u32 {
        match port {
            0x80000000..=0x8000031F => { // overlay port
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
                        overlay_lock[overlay_number].framebuffer_pointer + 0x80000000
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
            0x80001000..=0x80002200 => { // disk controller port
                let address_or_id = (port & 0x00000FFF) as usize;
                let operation = (port & 0x0000F000) >> 8;

                match operation {
                    0x10 => {
                        // we're reading the current mount state of the specified disk id
                        if address_or_id > 3 {
                            panic!("invalid disk ID");
                        }
                        match &self.disk_controller.disk[address_or_id] {
                            Some(disk) => disk.size as u32, // return size if this disk is mounted
                            None => 0, // return 0 if this disk is not mounted
                        }
                    }
                    0x20 => {
                        // we're reading from the sector buffer
                        if address_or_id > 512 {
                            panic!("attempted to read past the end of the disk controller sector buffer");
                        }
                        self.disk_controller.sector_buffer[address_or_id] as u32
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
                        if word < 0x80000000 {
                            panic!("overlay framebuffer must be within shared memory");
                        }
                        overlay_lock[overlay_number].framebuffer_pointer = word - 0x80000000;
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
            0x80001000..=0x80004200 => { // disk controller port
                let address_or_id = (port & 0x00000FFF) as usize;
                let operation = (port & 0x0000F000) >> 8;

                match operation {
                    0x10 => {
                        // we're requesting a disk to be mounted to the specified disk id
                        if address_or_id > 3 {
                            panic!("invalid disk ID");
                        }
                        let file = self.disk_controller.select_file();
                        match file {
                            Some(file) => self.disk_controller.mount(file, address_or_id as u8),
                            None => {},
                        };
                    }
                    0x20 => {
                        // we're writing to the sector buffer
                        if address_or_id > 512 {
                            panic!("attempted to read past the end of the disk controller sector buffer");
                        }
                        self.disk_controller.sector_buffer[address_or_id] = word as u8;
                    }
                    0x30 => {
                        // we're reading the specified sector of the specified disk id into the sector buffer
                        if address_or_id > 3 {
                            panic!("invalid disk ID");
                        }
                        self.disk_controller.set_current_sector(address_or_id as u8, word);
                        self.disk_controller.read_into_buffer(address_or_id as u8);
                    }
                    0x40 => {
                        // we're writing the specified sector to the specified disk id from the sector buffer
                        if address_or_id > 3 {
                            panic!("invalid disk ID");
                        }
                        self.disk_controller.set_current_sector(address_or_id as u8, word);
                        self.disk_controller.write_from_buffer(address_or_id as u8);
                    }
                    _ => panic!("invalid disk controller port"),
                }
            }
            _ => return,
        }
    }
}