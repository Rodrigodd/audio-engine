use audio_engine::{AudioEngine, OggDecoder, SineWave};
use std::{io::Cursor, time::Instant};

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
        .new_sound(SineWave::new(engine.sample_rate(), 500.0))
        .unwrap();
    track2.set_loop(true);
    track2.play();

    let mut time: f32;
    let start_time = Instant::now();
    loop {
        time = (Instant::now() - start_time).as_secs_f32();

        track2.set_volume(
            time.sin() * 0.5 + 0.5, // [-1, 1] -> [0, 1]
        );
    }
}
