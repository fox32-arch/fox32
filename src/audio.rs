// audio.rs

use crate::Interrupt;
use crate::memory::Memory;

use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;
use std::{thread, time};
use rodio::{OutputStream, buffer::SamplesBuffer, Sink};

const AUDIO_BUFFER_0_ADDRESS: usize = 0x0212C000;
const AUDIO_BUFFER_1_ADDRESS: usize = 0x02134000;
const AUDIO_BUFFER_2_ADDRESS: usize = 0x02290000;
const AUDIO_BUFFER_3_ADDRESS: usize = 0x02298000;
const AUDIO_BUFFER_SIZE: usize = 0x8000;

const AUDIO_CHANNEL_0_INTERRUPT: u8 = 0xFE;
const AUDIO_CHANNEL_1_INTERRUPT: u8 = 0xFD;
const AUDIO_CHANNEL_2_INTERRUPT: u8 = 0xFC;
const AUDIO_CHANNEL_3_INTERRUPT: u8 = 0xFB;

pub struct AudioChannel {
    pub id: u8,
    pub playing: bool,
    pub sample_rate: u32,
}

impl AudioChannel {
    pub fn new(id: u8) -> Self {
        AudioChannel {
            id,
            playing: false,
            sample_rate: 0,
        }
    }

    // this is terrible
    pub fn spawn_thread(this: Arc<Mutex<Self>>, interrupt_sender: Sender<Interrupt>, memory: Memory) {
        let (audio_buffer_address, audio_channel_interrupt) = match this.lock().unwrap().id {
            0 => (AUDIO_BUFFER_0_ADDRESS, AUDIO_CHANNEL_0_INTERRUPT),
            1 => (AUDIO_BUFFER_1_ADDRESS, AUDIO_CHANNEL_1_INTERRUPT),
            2 => (AUDIO_BUFFER_2_ADDRESS, AUDIO_CHANNEL_2_INTERRUPT),
            3 => (AUDIO_BUFFER_3_ADDRESS, AUDIO_CHANNEL_3_INTERRUPT),
            _ => unreachable!(),
        };
        let builder = thread::Builder::new().name("audio".to_string());
        builder.spawn({
            move || {
                let (_stream, stream_handle) = OutputStream::try_default().unwrap();
                let channel = Sink::try_new(&stream_handle).unwrap();
                loop {
                    // every `sleep` number of ms, play what is in the audio buffer
                    let self_lock = this.lock().unwrap();
                    if self_lock.playing {
                        let audio_buffer: Vec<i16> = memory.ram()[audio_buffer_address..audio_buffer_address+AUDIO_BUFFER_SIZE]
                                                    .to_vec()
                                                    .chunks_exact(2)
                                                    .map(|x| ((x[1] as i16) << 8) | x[0] as i16)
                                                    .collect();
                        // 1 Hz = 1000 ms
                        let sleep: f32 = (1000 as f32 / self_lock.sample_rate as f32) * (AUDIO_BUFFER_SIZE as f32 / 2.0);
                        channel.append(SamplesBuffer::new(1, self_lock.sample_rate, audio_buffer));
                        interrupt_sender.send(Interrupt::Request(audio_channel_interrupt)).unwrap();
                        thread::sleep(time::Duration::from_millis(sleep as u64));
                    }
                }
            }
        }).unwrap();
    }
}
