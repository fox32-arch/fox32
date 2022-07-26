// main.rs

pub mod memory;
pub mod audio;
pub mod bus;
pub mod cpu;
pub mod keyboard;
pub mod mouse;
pub mod disk;

use audio::Audio;
use bus::Bus;
use cpu::{Cpu, Interrupt};
use keyboard::Keyboard;
use mouse::Mouse;
use disk::DiskController;
use memory::{MEMORY_RAM_START, MEMORY_ROM_START, MemoryRam, Memory};

use std::sync::{Arc, mpsc, Mutex};
use std::thread;
use std::time;
use std::process::exit;
use std::env;
use std::fs::{File, read};

use image;
use log::error;
use pixels::{Pixels, SurfaceTexture};
use rodio::{OutputStream, buffer::SamplesBuffer, Sink};
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{WindowBuilder, Icon};
use winit_input_helper::WinitInputHelper;

const WIDTH: usize = 640;
const HEIGHT: usize = 480;

const FRAMEBUFFER_ADDRESS: usize = 0x02000000;
const AUDIO_BUFFER_0_ADDRESS: usize = 0x0212C000;
const AUDIO_BUFFER_1_ADDRESS: usize = 0x02134000;
const AUDIO_BUFFER_SIZE: usize = 0x8000;

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

fn read_rom() -> Vec<u8> {
    read("fox32.rom").unwrap_or_else(|_| {
        println!("fox32.rom not found, attempting to open ../fox32rom/fox32.rom instead");
        read("../fox32rom/fox32.rom").unwrap_or_else(|_| {
            error("fox32.rom not found!");
        })
    })
}

pub fn error(message: &str) -> ! {
    println!("Error: {}", message);
    exit(1);
}

pub fn warn(message: &str) {
    println!("Warning: {}", message);
}

fn main() {
    let version_string = format!("fox32 {} ({})", env!("VERGEN_BUILD_SEMVER"), env!("VERGEN_GIT_SHA_SHORT"));
    println!("{}", version_string);

    let args: Vec<String> = env::args().collect();
    /*if args.len() != 2 {
        println!("fox32\nUsage: {} <binary>", &args[0]);
        exit(1);
    }*/

    let mut display = Display::new();
    let keyboard = Arc::new(Mutex::new(Keyboard::new()));
    let mouse = Arc::new(Mutex::new(Mouse::new()));

    let audio = Arc::new(Mutex::new(Audio::new()));

    let memory = Memory::new(read_rom().as_slice());
    let mut bus = Bus {
        memory: memory.clone(),
        audio: audio.clone(),
        disk_controller: DiskController::new(),
        keyboard: keyboard.clone(),
        mouse: mouse.clone(),
        overlays: display.overlays.clone(),
    };

    if args.len() > 1 {
        let mut args_iter = args.iter();
        args_iter.next();
        for (i, arg) in args_iter.enumerate() {
            bus.disk_controller.insert(File::open(&arg).expect("failed to load provided disk image"), i as u8);
        }
    }

    let memory_audio = memory.clone();
    let memory_cpu = memory.clone();
    let memory_eventloop = memory.clone();

    let ram_size = memory_cpu.ram().len();
    let ram_bottom_address = MEMORY_RAM_START;
    let ram_top_address = ram_bottom_address + ram_size - 1;
    println!("RAM: {:.2} MiB mapped at {:#010X}-{:#010X}", ram_size / 1048576, ram_bottom_address, ram_top_address);

    let rom_size = memory_cpu.rom().len();
    let rom_bottom_address = MEMORY_ROM_START;
    let rom_top_address = rom_bottom_address + rom_size - 1;
    println!("ROM: {:.2} KiB mapped at {:#010X}-{:#010X}", rom_size / 1024, rom_bottom_address, rom_top_address);

    let mut cpu = Cpu::new(bus);

    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let icon = image::load_from_memory(include_bytes!("32.png")).unwrap();
    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        WindowBuilder::new()
            .with_title(version_string)
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_window_icon(Some(Icon::from_rgba(icon.as_bytes().to_vec(), 128, 128).unwrap()))
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
                if !cpu.flag.interrupt {
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

    let interrupt_sender_audio = interrupt_sender.clone();
    let builder = thread::Builder::new().name("audio".to_string());
    builder.spawn({
        move || {
            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            let sink = Sink::try_new(&stream_handle).unwrap();
            loop {
                // every 500 ms, play what is in the audio buffer and tell fox32 to swap them
                let mut audio_lock = audio.lock().unwrap();
                if audio_lock.playing {
                    let current_buffer: Vec<i16> = if audio_lock.current_buffer_is_0 {
                        audio_lock.current_buffer_is_0 = false;
                        memory_audio.ram()[AUDIO_BUFFER_0_ADDRESS..AUDIO_BUFFER_0_ADDRESS+AUDIO_BUFFER_SIZE].to_vec().chunks_exact(2).map(|x| ((x[1] as i16) << 8) | x[0] as i16).collect()
                    } else {
                        audio_lock.current_buffer_is_0 = true;
                        memory_audio.ram()[AUDIO_BUFFER_1_ADDRESS..AUDIO_BUFFER_1_ADDRESS+AUDIO_BUFFER_SIZE].to_vec().chunks_exact(2).map(|x| ((x[1] as i16) << 8) | x[0] as i16).collect()
                    };
                    let buffer = SamplesBuffer::new(1, 22050, current_buffer);
                    sink.append(buffer);
                    drop(audio_lock);
                    interrupt_sender_audio.send(Interrupt::Request(0xFE)).unwrap(); // audio interrupt, swap audio buffers
                    thread::sleep(time::Duration::from_millis(500));
                }
            }
        }
    }).unwrap();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        // draw the current frame
        if let Event::MainEventsCleared = event {
            // update internal state and request a redraw

            match interrupt_sender.send(Interrupt::Request(0xFF)) { // vsync interrupt
                Ok(_) => {},
                Err(_) => {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            };
            display.update(memory_eventloop.clone().ram());
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
        if let Event::WindowEvent { ref event, .. } = event {
            if let WindowEvent::KeyboardInput { input, .. } = event {
                let mut keyboard_lock = keyboard.lock().unwrap();
                let mut scancode = input.scancode;
                if input.state == ElementState::Released {
                    scancode |= 0x80; // "break" scancode
                }
                //println!("scancode: {:x}", scancode);
                keyboard_lock.push(scancode);
            }
        }

        if input.update(&event) {
            // close events
            if input.quit() {
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
            if mouse_lock.held && !input.mouse_held(0) {
                // mouse button was released this frame
                mouse_lock.released = true;
            }
            mouse_lock.held = input.mouse_held(0);
            if input.mouse_pressed(0) {
                mouse_lock.clicked = true;
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

    fn update(&mut self, ram: &MemoryRam) {
        let overlay_lock = self.overlays.lock().unwrap();

        for i in 0..(HEIGHT*WIDTH*4) as usize {
            self.background[i] = ram[FRAMEBUFFER_ADDRESS + i];
        }

        for index in 0..=31 {
            if overlay_lock[index].enabled {
                blit_overlay(&mut self.background, &overlay_lock[index], ram);
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
fn blit_overlay(framebuffer: &mut [u8], overlay: &Overlay, ram: &[u8]) {
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

    let overlay_framebuffer = &ram[(overlay.framebuffer_pointer as usize)..((overlay.framebuffer_pointer+((width as u32)*(height as u32))) as usize)];

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
