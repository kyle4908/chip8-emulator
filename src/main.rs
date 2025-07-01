mod emulator;
mod opcode;

use crate::emulator::{SCREEN_HEIGHT, SCREEN_WIDTH};
use clap::Parser;
use emulator::Emulator;
use log::{debug, info};
use sdl2::event::Event;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::TextureAccess;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(version, long_about = None)]
struct Args {
    /// Name of the file to run in the emulator
    #[arg(short, long)]
    filename: String,

    /// Older versions of the CHIP-8 used to set the X register to the value of the Y register
    /// on shift operations. Newer versions shift X in place, if you need the older behaviour
    /// use this flag.
    #[arg(short, long)]
    shift_with_y: bool,

    /// On jump operations, the more common behaviour is to take the last 12 bits of the opcode
    /// and jump to the address of that plus the value in register 0. In newer versions, jump
    /// operations work by going to the address of the last 8 bits plus the value in register X.
    /// If you need the behaviour with the X register, use this flag.
    #[arg(short, long)]
    jump_with_x: bool,
}

fn main() -> Result<(), String> {
    let args = Args::parse();

    dotenvy::dotenv().map_err(|e| e.to_string())?;
    env_logger::init();

    info!("Running {}", args.filename);

    debug!("Running with {:?}", args);

    let pixel_size = 16;

    let context = sdl2::init()?;
    let video_subsystem = context.video()?;
    let window = video_subsystem
        .window(
            "Chip8-Emulator",
            (SCREEN_WIDTH * pixel_size) as u32,
            (SCREEN_HEIGHT * pixel_size) as u32,
        )
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

    let screen_area = Rect::new(
        0,
        0,
        (SCREEN_WIDTH * pixel_size) as u32,
        (SCREEN_HEIGHT * pixel_size) as u32,
    );

    let mut running = true;
    let mut event_pump = context.event_pump().map_err(|e| e.to_string())?;

    let texture_creator = canvas.texture_creator();

    let mut texture = texture_creator
        .create_texture(
            PixelFormatEnum::RGB332,
            TextureAccess::Streaming,
            SCREEN_WIDTH as u32,
            SCREEN_HEIGHT as u32,
        )
        .map_err(|e| e.to_string())?;

    let mut emu = Emulator::new(args.shift_with_y, args.jump_with_x);
    emu.load_file(&args.filename);

    canvas.set_draw_color(Color::BLACK);
    canvas.fill_rect(screen_area)?;
    canvas.present();

    while running {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    running = false;
                }
                _ => {}
            }
        }
        emu.execute();
        if emu.needs_redraw() {
            texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
                for y in 0..SCREEN_HEIGHT {
                    for x in 0..SCREEN_WIDTH {
                        let offset = y * pitch + x;
                        buffer[offset] = if emu.screen[y][x] {
                            0xFF // white
                        } else {
                            0x00 // black
                        };
                    }
                }
            })?;
            canvas.clear();
            canvas.copy(
                &texture,
                None,
                Some(Rect::new(
                    0,
                    0,
                    (SCREEN_WIDTH * pixel_size) as u32,
                    (SCREEN_HEIGHT * pixel_size) as u32,
                )),
            )?;
            canvas.present();
        }
        std::thread::sleep(Duration::new(0, 140000));
    }

    Ok(())
}
