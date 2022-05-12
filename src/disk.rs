// disk.rs

use crate::memory::MemoryRam;

use std::io::{Seek, SeekFrom, Read, Write};
use std::fs::File;
use rfd::FileDialog;

pub struct Disk {
    file: File,
    size: u64,
    current_sector: u32,
}

impl Disk {
    pub fn new(file: File) -> Self {
        Disk {
            size: file.metadata().unwrap().len(),
            file,
            current_sector: 0,
        }
    }

    pub fn size(&self) -> u64 {
        self.size
    }
}

pub struct DiskController {
    pub disk: [Option<Disk>; 4],
    pub buffer_pointer: usize,
}

impl DiskController {
    pub fn new() -> Self {
        Self {
            disk: [None, None, None, None],
            buffer_pointer: 0x00000000
        }
    }

    pub fn select_file(&self) -> Option<File> {
        let path = FileDialog::new()
            .add_filter("Disk Image", &["img", "dsk"])
            .add_filter("f32 Binary", &["f32"])
            .add_filter("Raw Binary", &["bin"])
            .add_filter("All Files", &["*"])
            .set_title(&format!("Select a file to insert"))
            .pick_file();
        match path {
            Some(path) => Some(File::open(path).expect("failed to open disk image")),
            None => None,
        }
    }

    pub fn insert(&mut self, file: File, disk_id: u8) {
        self.disk[disk_id as usize] = Some(Disk::new(file));
    }
    pub fn remove(&mut self, disk_id: u8) {
        self.disk[disk_id as usize] = None;
    }

    fn disk_mut(&mut self, disk_id: u8) -> &mut Disk {
        self.disk[disk_id as usize].as_mut().expect("attempted to access unmounted disk")
    }

    pub fn get_size(&mut self, disk_id: u8) -> u64 {
        self.disk_mut(disk_id).size
    }
    pub fn get_current_sector(&mut self, disk_id: u8) -> u32 {
        self.disk_mut(disk_id).current_sector
    }
    pub fn set_current_sector(&mut self, disk_id: u8, sector: u32) {
        let disk = self.disk_mut(disk_id);
        disk.current_sector = sector;
        disk.file.seek(SeekFrom::Start(sector as u64 * 512)).expect("attempted to seek to sector beyond edge of disk");
    }

    pub fn read_into_memory(&mut self, disk_id: u8, ram: &mut MemoryRam) -> usize {
        let disk = self.disk_mut(disk_id);
        let mut temp_buffer = [0u8; 512];

        let number_of_bytes_read = disk.file.read(&mut temp_buffer).unwrap();
        ram[self.buffer_pointer..self.buffer_pointer+512].copy_from_slice(&temp_buffer);
        number_of_bytes_read
    }
    pub fn write_from_memory(&mut self, disk_id: u8, ram: &MemoryRam) -> usize {
        let disk = self.disk[disk_id as usize].as_mut().expect("attempted to access unmounted disk");

        let number_of_bytes_written = disk.file.write(ram.get(self.buffer_pointer..self.buffer_pointer+512).unwrap()).unwrap();
        number_of_bytes_written
    }
}
