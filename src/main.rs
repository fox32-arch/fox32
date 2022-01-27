// main.rs

pub mod cpu;
pub mod mouse;
use cpu::{Bus, CPU, Memory, Interrupt, IO};
use mouse::Mouse;

use std::convert::TryInto;
use std::env;
use std::fs::read;
//use std::process::exit;
use std::sync::{Arc, mpsc, Mutex};
use std::io::{stdout, Write};
use std::thread;

use log::error;
use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use rodio::{OutputStream, buffer::SamplesBuffer, Sink};

const WIDTH: usize = 640;
const HEIGHT: usize = 480;

pub struct Display {
    background: Vec<u8>,
    overlays: Arc<Mutex<Vec<Overlay>>>,
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Overlay {
    enabled: bool,
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    framebuffer_pointer: u32,
}

impl IO for Bus {
    fn read_io(&mut self, port: u32) -> u32 {
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
    fn write_io(&mut self, port: u32, word: u32) {
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

fn main() {
    let args: Vec<String> = env::args().collect();
    /*if args.len() != 2 {
        println!("fox32\nUsage: {} <binary>", &args[0]);
        exit(1);
    }*/

    let mut display = Display::new();
    let mouse = Arc::new(Mutex::new(Mouse::new()));

    // 32 MiB of shared memory
    // add some "bonus bytes" at the end of memory to prevent dumb errors - (lua was here)
    // see the ImmediatePtr match arm in read_source() in cpu.rs for more info
    // basically all immediate pointers read 32 bits, even if the opcode size is smaller
    // so attempting to read something like the last byte of shared memory (0x03FFFFFF) would previously panic
    let shared_memory = Arc::new(Mutex::new(vec![0u8; 0x0200000F]));

    let mut cpu = {
        let cpu_shared_memory = Arc::clone(&shared_memory);
        let cpu_overlays = Arc::clone(&display.overlays);
        let cpu_read_only_memory = read("fox32.rom").expect("fox32.rom not found!");
        // 32 MiB of CPU-only memory
        let memory = Memory::new(0x02000000, cpu_shared_memory, cpu_overlays, cpu_read_only_memory);

        let cpu_mouse = Arc::clone(&mouse);

        let bus = Bus { memory, mouse: cpu_mouse };
        CPU::new(bus)
    };

    if args.len() >= 2 {
        let input_file_name = &args[1];
        let opcodes = read(input_file_name).unwrap();
        //println!("{:02X?}", opcodes);
        let mut address = 0u32;
        for byte in opcodes.iter() {
            cpu.bus.memory.write_8(address, *byte);
            address += 1;
        }
    }

    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        WindowBuilder::new()
            .with_title("fox32")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    window.set_cursor_visible(false);

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH as u32, HEIGHT as u32, surface_texture).unwrap()
    };

    let (interrupt_sender, interrupt_receiver) = mpsc::channel::<Interrupt>();

    let builder = thread::Builder::new().name("cpu".to_string());
    builder.spawn({
        move || {
            loop {
                while !cpu.halted {
                    if let Ok(interrupt) = interrupt_receiver.try_recv() {
                        cpu.interrupt(interrupt);
                    }
                    cpu.execute_memory_instruction();
                }
                if !cpu.interrupts_enabled {
                    // the cpu was halted and interrupts are disabled
                    // at this point, the cpu is dead and cannot resume, break out of the loop
                    break;
                }
                if let Ok(interrupt) = interrupt_receiver.recv() {
                    cpu.halted = false;
                    cpu.interrupt(interrupt);
                } else {
                    // sender is closed, break
                    break;
                }
            }
            println!("CPU halted");
        }
    }).unwrap();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        // draw the current frame
        if let Event::MainEventsCleared = event {
            // update internal state and request a redraw

            let mut shared_memory_lock = shared_memory.lock().expect("failed to lock the shared memory mutex");
            //shared_memory_lock[0x01FFFFFF] = shared_memory_lock[0x01FFFFFF].overflowing_add(1).0; // increment vsync counter
            let _ = interrupt_sender.send(Interrupt::Request(0xFF)); // vsync interrupt
            display.update(&mut *shared_memory_lock);
            drop(shared_memory_lock);
            window.request_redraw();

            display.draw(pixels.get_frame());
            if pixels
                .render()
                .map_err(|e| error!("pixels.render() failed: {}", e))
                .is_err()
            {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        // handle input events
        if input.update(&event) {
            // close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            // resize the window
            if let Some(size) = input.window_resized() {
                pixels.resize_surface(size.width, size.height);
            }

            let mouse_pixel = input.mouse().map(|(mx, my)| {
                let (x, y) = pixels.window_pos_to_pixel((mx, my)).unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));
                (x as u16, y as u16)
            }).unwrap_or_default();

            // TODO: allow right click
            let mut mouse_lock = mouse.lock().expect("failed to lock the mouse mutex");
            mouse_lock.x = mouse_pixel.0;
            mouse_lock.y = mouse_pixel.1;
            mouse_lock.held = input.mouse_held(0);
            if input.mouse_pressed(0) {
                println!("Mouse click at {:?}", mouse_pixel);
                mouse_lock.click = true;
            }
        }
    });
}

impl Display {
    fn new() -> Self {
        Self {
            background: vec![0; (HEIGHT*WIDTH*4) as usize],
            overlays: Arc::new(Mutex::new(vec![Overlay { enabled: false, width: 16, height: 16, x: 0, y: 0, framebuffer_pointer: 0 }; 32])),
        }
    }

    fn update(&mut self, shared_memory: &mut [u8]) {
        let overlay_lock = self.overlays.lock().unwrap();

        for i in 0..(HEIGHT*WIDTH*4) as usize {
            self.background[i] = shared_memory[i];
        }

        for index in 0..=31 {
            if overlay_lock[index].enabled {
                blit_overlay(&mut self.background, &overlay_lock[index], shared_memory);
            }
        }
    }

    fn draw(&self, frame: &mut [u8]) {
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            //let x = (i % WIDTH as usize) as i16;
            //let y = (i / WIDTH as usize) as i16;

            let i = i * 4;

            let slice = &self.background[i..i+4];
            pixel.copy_from_slice(slice);
        }
    }
}

// modified from https://github.com/parasyte/pixels/blob/main/examples/invaders/simple-invaders/src/sprites.rs
fn blit_overlay(framebuffer: &mut [u8], overlay: &Overlay, shared_memory: &mut [u8]) {
    //assert!(overlay.x + overlay.width <= WIDTH);
    //assert!(overlay.y + overlay.height <= HEIGHT);

    let mut width = overlay.width * 4;
    let mut height = overlay.height;

    // FIXME: this is a hack, and it only allows overlays to go off-screen on the bottom and right sides
    //        it also completely fucks up the image on the right side :p
    if overlay.x + overlay.width > WIDTH {
        let difference = (overlay.x + overlay.width) - WIDTH;
        width = width - (difference * 4);
        //println!("width: {}, difference: {}", width, difference);
    }
    if overlay.y + overlay.height > HEIGHT {
        let difference = (overlay.y + overlay.height) - HEIGHT;
        height = height - difference;
        //println!("height: {}, difference: {}", height, difference);
    }

    let overlay_framebuffer = &shared_memory[overlay.framebuffer_pointer as usize..(overlay.framebuffer_pointer+((width as u32)*(height as u32))) as usize];

    let mut overlay_framebuffer_index = 0;
    for y in 0..height {
        let index = overlay.x * 4 + overlay.y * WIDTH * 4 + y * WIDTH * 4;
        // merge overlay onto screen
        // this is such a dumb hack
        let mut zipped = framebuffer[index..index + width].iter_mut().zip(&overlay_framebuffer[overlay_framebuffer_index..overlay_framebuffer_index + width]);
        while let Some((left, &right)) = zipped.next() {
            let (left_0, &right_0) = (left, &right);
            let (left_1, &right_1) = zipped.next().unwrap();
            let (left_2, &right_2) = zipped.next().unwrap();
            let (left_3, &right_3) = zipped.next().unwrap();
            // ensure that the alpha byte is greater than zero, meaning that this pixel shouldn't be transparent
            if right_3 > 0 {
                *left_0 = right_0;
                *left_1 = right_1;
                *left_2 = right_2;
                *left_3 = right_3;
            }
        }
        overlay_framebuffer_index += width;
    }
}
