// mouse.rs

pub struct Mouse {
    pub x: u16,
    pub y: u16,
    pub clicked: bool,
    pub released: bool,
    pub held: bool,
}

impl Mouse {
    pub fn new() -> Self {
        Mouse { x: 0, y: 0, clicked: false, released: false, held: false }
    }
}