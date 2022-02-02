// main.rs

pub mod bus;
pub mod cpu;
pub mod disk;
pub mod memory;
pub mod mouse;
use bus::Bus;
use cpu::{Cpu, Interrupt};
use disk::DiskController;
use memory::Memory;
use mouse::Mouse;

use std::env;
use std::fs::read;
//use std::process::exit;
use std::sync::{Arc, mpsc, Mutex};
use std::thread;

use log::error;
use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

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

fn read_rom() -> Vec<u8> {
    read("fox32.rom").unwrap_or_else(|_| {
        println!("fox32.rom not found, attempting to open ../fox32rom/fox32.rom instead");
        read("../fox32rom/fox32.rom").unwrap_or_else(|_| {
            panic!("oh fuck");
        })
    })
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
    let mouse = Arc::new(Mutex::new(Mouse::new()));

    // 32 MiB of shared memory
    // plus 3 bytes at the end to allow for reading the last byte of memory as an immediate pointer
    // Operand::ImmediatePtr always reads 4 bytes
    let shared_memory = Arc::new(Mutex::new(vec![0u8; 0x02000003]));

    let mut cpu = {
        // 32 MiB of fast memory
        // plus 3 extra bytes at the end to allow for reading the last byte of memory as an immediate pointer
        // Operand::ImmediatePtr always reads 4 bytes
        let cpu_fast_memory = vec![0; 0x02000003];
        let cpu_shared_memory = Arc::clone(&shared_memory);
        let cpu_overlays = Arc::clone(&display.overlays);
        let cpu_read_only_memory = read_rom();

        let fast_size = cpu_fast_memory.len() - 3;
        let fast_bottom_address = 0x00000000;
        let fast_top_address = fast_bottom_address + fast_size - 1;
        println!("Fast:   {:.2}MB mapped at {:#010X}-{:#010X}", fast_size / 1048576, fast_bottom_address, fast_top_address);

        let shared_size = { cpu_shared_memory.lock().unwrap().len() - 3 };
        let shared_bottom_address = 0x80000000;
        let shared_top_address = shared_bottom_address + shared_size - 1;
        println!("Shared: {:.2}MB mapped at {:#010X}-{:#010X}", shared_size / 1048576, shared_bottom_address, shared_top_address);

        let rom_size = cpu_read_only_memory.len();
        let rom_bottom_address = 0xF0000000;
        let rom_top_address = rom_bottom_address + rom_size - 1;
        println!("ROM:    {:.2}MB mapped at {:#010X}-{:#010X}", rom_size / 1048576, rom_bottom_address, rom_top_address);

        let memory = Memory::new(cpu_fast_memory, cpu_shared_memory, cpu_overlays, cpu_read_only_memory);

        let cpu_mouse = Arc::clone(&mouse);

        let disk_controller = DiskController::new();
        let bus = Bus { disk_controller, memory, mouse: cpu_mouse };
        Cpu::new(bus)
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
            .with_title(version_string)
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
