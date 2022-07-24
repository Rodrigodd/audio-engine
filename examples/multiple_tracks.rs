use audio_engine::{AudioEngine, OggDecoder};
use std::{
    io::Cursor,
    time::Instant
};

fn log_panic() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let current = std::thread::current();
        let thread = current.name().unwrap_or("unnamed");

        let msg = if let Some(s) = info.payload().downcast_ref::<&'static str>() {
            *s
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            &**s
        } else {
            "Box<Any>"
        };

        match info.location() {
            Some(location) => {
                log::error!(
                    "thread '{}' panicked at '{}': {}:{}",
                    thread,
                    msg,
                    location.file(),
                    location.line(),
                );
            }
            None => {
                log::error!("thread '{}' panicked at '{}'", thread, msg,)
            }
        }

        default_hook(info);
    }));
}

struct SineSource {
    i: u32,
    freq: f32
}

impl SineSource {
    fn new (freq: f32) -> Self {
        Self { i: 0, freq }
    }
}

impl audio_engine::SoundSource for SineSource {
    fn sample_rate(&self) -> u32 {
        44100
    }
    fn channels(&self) -> u16 {
        1
    }
    fn reset(&mut self) {
        self.i = 0
    }
    fn write_samples(&mut self, out: &mut [i16]) -> usize {
        for o in out.iter_mut() {
            let t = self.i as f32 / self.sample_rate() as f32;
            let amplitude = (i16::max_value() / 4) as f32;
            *o = ((self.freq * std::f32::consts::TAU * t).sin() * amplitude) as i16;
            self.i += 1;
        }
        out.len()
    }
}

#[cfg_attr(
    target_os = "android",
    ndk_glue::main(
        backtrace = "on",
        ndk_glue = "ndk_glue",
        logger(level = "trace", tag = "audio-engine", filter = "debug")
    )
)]
fn main() {
    #[cfg(not(target_os = "android"))]
    env_logger::init();

    log_panic();

    let engine = AudioEngine::new().unwrap();

    let mut track1 = engine
        .new_sound(OggDecoder::new(Cursor::new(&include_bytes!("pipe.ogg")[..])).unwrap())
        .unwrap();
    track1.set_loop(true);
    track1.play();

    let mut track2 = engine
        .new_sound(SineSource::new(500.0))
        .unwrap();
    track2.set_loop(true);
    track2.play();


    let mut time: f32;
    let start_time = Instant::now();
    loop {
        time = (Instant::now() - start_time).as_secs_f32();

        track2.set_volume(
            time.sin() * 0.5 + 0.5 // [-1, 1] -> [0, 1]
        );
    }
}
