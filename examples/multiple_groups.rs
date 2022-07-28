use audio_engine::{AudioEngine, SineWave};
use std::time::Instant;

fn main() {
    env_logger::init();

    #[derive(Eq, Hash, PartialEq)]
    enum Groups {
        Minor,
        Major,
    }

    let engine = AudioEngine::with_groups::<Groups>().unwrap();

    // the freq ration of a semitone in twelve-tone equal temperament.
    let s = 2.0_f32.powf(1.0 / 12.0);
    // the note A.
    let la = 440.0;

    let la_minor = [la, la * s.powi(3), la * s.powi(7)];
    la_minor.map(|freq| {
        let mut track = engine
            .new_sound_with_group(Groups::Minor, SineWave::new(engine.sample_rate(), freq))
            .unwrap();
        track.set_loop(true);
        track.set_volume(0.3);
        track.play();
        track
    });

    let la_maior = [la, la * s.powi(4), la * s.powi(7)];
    la_maior.map(|freq| {
        let mut track = engine
            .new_sound_with_group(Groups::Major, SineWave::new(engine.sample_rate(), freq))
            .unwrap();
        track.set_loop(true);
        track.set_volume(0.3);
        track.play();
        track
    });

    let mut time: f32;
    let start_time = Instant::now();
    loop {
        time = (Instant::now() - start_time).as_secs_f32();

        engine.set_group_volume(
            Groups::Minor,
            time.sin() * 0.5 + 0.5, // [-1, 1] -> [0, 1]
        );

        engine.set_group_volume(
            Groups::Major,
            time.sin() * 0.5 - 0.5, // [-1, 1] -> [1, 0]
        );
    }
}
