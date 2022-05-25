// audio.rs

pub struct Audio {
    pub current_buffer_is_0: bool,
    pub playing: bool,
}

impl Audio {
    pub fn new() -> Self {
        Audio {
            current_buffer_is_0: true,
            playing: false,
        }
    }
}
