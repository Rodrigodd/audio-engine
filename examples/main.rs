use audio_engine::{ AudioEngine, OggDecoder, WavDecoder };
use std::io::Cursor;

fn main() {
    let mut engine = AudioEngine::new().unwrap();
    let mut sounds = [
        engine.new_sound(
            OggDecoder::new(Cursor::new(&include_bytes!("pipe.ogg")[..]))
        ).unwrap(),
        engine.new_sound(
            WavDecoder::new(Cursor::new(&include_bytes!("ilussion.wav")[..]))
        ).unwrap(),
        engine.new_sound(
            WavDecoder::new(Cursor::new(&include_bytes!("0.wav")[..]))
        ).unwrap(),
    ];
    sounds[0].set_loop(true);
    sounds[1].set_loop(true);
    sounds[2].set_loop(true);

    use std::io::Write;
    let mut stdout = std::io::stdout();
    let mut sound_stack = Vec::new();
    let stdin = std::io::stdin();
    let mut line = String::new();
    loop {
        print!(">> ");
        stdout.flush().unwrap();
        stdin.read_line(&mut line).unwrap();
        let mut args = line.split_ascii_whitespace();
        let command = args.next();
        let index = args.next().and_then(|x| x.parse::<usize>().ok()).unwrap_or(0);
        match command {
            Some("play") => sounds[index].play(),
            Some("pause") => sounds[index].pause(),
            Some("stop") => sounds[index].stop(),
            Some("reset") => sounds[index].reset(),
            Some("exit") => break,
            Some("volume") => sounds[index].set_volume(args.next().and_then(|x| x.parse::<f32>().ok()).unwrap_or(0.0)),
            Some("new") => {
                sound_stack.push(engine.new_sound(
                    OggDecoder::new(Cursor::new(&include_bytes!("pipe.ogg")[..]))
                ).unwrap());
                sound_stack.last_mut().unwrap().play();
            }
            _ => println!(" invalid command"),
        }
        line.clear();
    }
}