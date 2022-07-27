use audio_engine::{AudioEngine, OggDecoder, WavDecoder};
use std::path::PathBuf;

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

    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let mut file = None;
    let mut looping = false;

    for arg in args {
        if arg.starts_with("-") {
            if arg == "--loop" || arg == "-l" {
                looping = true;
            } else {
                eprintln!("unexpected option {}", arg);
                std::process::exit(2);
            }
        } else {
            file = Some(arg);
        }
    }

    let path = match file {
        Some(x) => x,
        None => {
            eprintln!("expected path to file as argument");
            std::process::exit(1);
        }
    };
    let path = PathBuf::from(path);

    let file = match std::fs::File::open(&path) {
        Ok(x) => x,
        Err(err) => {
            eprintln!("error loading file: {}", err);
            std::process::exit(1);
        }
    };
    let buffered = std::io::BufReader::new(file);

    let engine = AudioEngine::new().unwrap();
    let mut music = match path.extension() {
        Some(x) if x == "wav" => engine
            .new_sound(WavDecoder::new(buffered).unwrap())
            .unwrap(),
        Some(x) if x == "ogg" => engine
            .new_sound(OggDecoder::new(buffered).unwrap())
            .unwrap(),
        Some(x) => {
            eprintln!(
                "unsupported file format {}",
                format!("'{}'", x.to_string_lossy())
            );
            std::process::exit(3);
        }
        _ => {
            eprintln!("unkown file format",);
            std::process::exit(3);
        }
    };

    music.set_loop(looping);
    music.play();

    loop {}
}
