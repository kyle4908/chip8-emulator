/// Largely directly from docs of SDL2 bindings for Rust, I don't really understand a lot of it
/// https://docs.rs/sdl2/latest/sdl2/audio/index.html
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};
use sdl2::Sdl;

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // Generate a square wave
        for x in out.iter_mut() {
            *x = if self.phase <= 0.5 {
                self.volume
            } else {
                -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

pub struct SoundSystem {
    device: AudioDevice<SquareWave>,
}

impl SoundSystem {
    pub fn new(sdl_context: Sdl) -> Self {
        let audio = sdl_context.audio().unwrap();
        let desired_spec = AudioSpecDesired {
            freq: Some(44100),
            channels: Some(1), // mono
            samples: None,     // default sample size
        };

        Self {
            device: audio
                .open_playback(None, &desired_spec, |spec| {
                    // initialize the audio callback
                    SquareWave {
                        phase_inc: 440.0 / spec.freq as f32,
                        phase: 0.0,
                        volume: 0.10,
                    }
                })
                .unwrap(),
        }
    }

    /// Resume beeping if sound timer greater than 0, pause otherwise
    pub fn handle_sound_timer(&self, timer: &u8) {
        if *timer > 0 {
            self.device.resume();
        } else {
            self.device.pause();
        }
    }
}
