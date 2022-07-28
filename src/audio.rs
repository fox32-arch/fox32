// audio.rs

pub struct Audio {
    pub current_buffer_is_0: bool,
    pub playing: bool,
    pub sample_rate: u32,
}

impl Audio {
    pub fn new() -> Self {
        Audio {
            current_buffer_is_0: true,
            playing: false,
            sample_rate: 0,
        }
    }
}
