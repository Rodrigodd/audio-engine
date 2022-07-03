use audio_engine::{AudioEngine, OggDecoder};
use std::io::Cursor;

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
    let mut music = engine
        .new_sound(OggDecoder::new(Cursor::new(&include_bytes!("pipe.ogg")[..])).unwrap())
        .unwrap();
    music.set_loop(true);
    music.play();

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
