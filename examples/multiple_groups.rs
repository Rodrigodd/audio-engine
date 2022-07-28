use audio_engine::AudioEngine;
use std::time::Instant;

struct SineSource {
    i: u32,
    freq: f32,
}

impl SineSource {
    fn new(freq: f32) -> Self {
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

fn main() {
    env_logger::init();

    #[derive(Eq, Hash, PartialEq)]
    enum Groups {
        Minor,
        Major,
    }

    let engine = AudioEngine::with_groups::<Groups>().unwrap();

    // a half-step ration in the scale
    let s = 2.0_f32.powf(1.0 / 12.0);
    // the note A.
    let la = 440.0;

    let la_minor = [la, la * s.powi(3), la * s.powi(7)];
    la_minor.map(|freq| {
        let mut track = engine
            .new_sound_with_group(Groups::Minor, SineSource::new(freq))
            .unwrap();
        track.set_loop(true);
        track.set_volume(0.3);
        track.play();
        track
    });

    let la_maior = [la, la * s.powi(4), la * s.powi(7)];
    la_maior.map(|freq| {
        let mut track = engine
            .new_sound_with_group(Groups::Major, SineSource::new(freq))
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
