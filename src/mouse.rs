// mouse.rs

pub struct Mouse {
    pub x: u16,
    pub y: u16,
    pub click: bool,
    pub held: bool,
}

impl Mouse {
    pub fn new() -> Self {
        Mouse { x: 0, y: 0, click: false, held: false }
    }
}