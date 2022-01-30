// disk.rs

use std::fs::File;
use std::io::{Seek, SeekFrom, Read, Write};
use rfd::FileDialog;

pub struct Disk {
    file: File,
    pub size: u64,
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
}

pub struct DiskController {
    pub disk: [Option<Disk>; 4],
    pub sector_buffer: [u8; 512],
}

impl DiskController {
    pub fn new() -> Self {
        DiskController {
            disk: [None, None, None, None],
            sector_buffer: [0; 512]
        }
    }
    pub fn select_file(&self) -> Option<File> {
        let path = FileDialog::new()
            .add_filter("disk image", &["img", "dsk"])
            .add_filter("f32 binary", &["f32"])
            .add_filter("raw binary", &["bin"])
            .pick_file();
        match path {
            Some(path) => Some(File::open(path).unwrap()),
            None => None,
        }
    }
    pub fn mount(&mut self, file: File, disk_id: u8) {
        self.disk[disk_id as usize] = Some(Disk::new(file));
    }
    pub fn unmount(&mut self, disk_id: u8) {
        self.disk[disk_id as usize] = None;
    }
    pub fn get_size(&mut self, disk_id: u8) -> u64 {
        let disk = self.disk[disk_id as usize].as_mut().expect("attempted to access unmounted disk");
        disk.size
    }
    pub fn set_current_sector(&mut self, disk_id: u8, sector: u32) {
        let disk = self.disk[disk_id as usize].as_mut().expect("attempted to access unmounted disk");
        disk.current_sector = sector;
        disk.file.seek(SeekFrom::Start(sector as u64 * 512)).unwrap();
    }
    pub fn get_current_sector(&mut self, disk_id: u8) -> u32 {
        let disk = self.disk[disk_id as usize].as_mut().expect("attempted to access unmounted disk");
        disk.current_sector
    }
    pub fn read_into_buffer(&mut self, disk_id: u8) -> usize {
        let disk = self.disk[disk_id as usize].as_mut().expect("attempted to access unmounted disk");
        let mut temp_buffer = [0u8; 512];

        let number_of_bytes_read = disk.file.read(&mut temp_buffer).unwrap();
        self.sector_buffer = temp_buffer;
        number_of_bytes_read
    }
    pub fn write_from_buffer(&mut self, disk_id: u8) -> usize {
        let disk = self.disk[disk_id as usize].as_mut().expect("attempted to access unmounted disk");

        let number_of_bytes_written = disk.file.write(&mut self.sector_buffer).unwrap();
        number_of_bytes_written
    }
}