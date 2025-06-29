mod emulator;
mod nibbles;

use crate::emulator::{SCREEN_HEIGHT, SCREEN_WIDTH};
use clap::Parser;
use emulator::Emulator;
use sdl2::event::Event;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(version, long_about = None)]
struct Args {
    /// Name of the file to run in the emulator
    #[arg(short, long)]
    filename: String,
}

fn main() -> Result<(), String> {
    let args = Args::parse();

    eprintln!("Playing {}", args.filename);

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
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    let screen_area = Rect::new(
        0,
        0,
        (SCREEN_WIDTH * pixel_size) as u32,
        (SCREEN_HEIGHT * pixel_size) as u32,
    );

    let mut running = true;
    let mut event_pump = context.event_pump().unwrap();

    let mut emu = Emulator::new();
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
            canvas.clear();

            for (y, row) in emu.screen.iter().enumerate() {
                for (x, &value) in row.iter().enumerate() {
                    if value {
                        canvas.set_draw_color(Color::WHITE);
                    } else {
                        canvas.set_draw_color(Color::BLACK);
                    }

                    let rect = Rect::new(
                        (x * pixel_size) as i32,
                        (y * pixel_size) as i32,
                        pixel_size as u32,
                        pixel_size as u32,
                    );
                    canvas.fill_rect(rect)?;
                }
            }
            canvas.present();
        }
        std::thread::sleep(Duration::new(0, 140000));
    }

    Ok(())
}
