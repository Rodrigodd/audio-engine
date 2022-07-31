use audio_engine::{Mixer, OggDecoder, SoundSource, WavDecoder};
use criterion::{criterion_group, criterion_main, Criterion};
use std::io::Cursor;

struct Nop;
impl SoundSource for Nop {
    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        44100
    }

    fn reset(&mut self) {}

    fn write_samples(&mut self, buffer: &mut [i16]) -> usize {
        buffer.len()
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("ogg decoder", |b| {
        b.iter(|| {
            let mut decoder =
                OggDecoder::new(Cursor::new(&include_bytes!("../examples/pipe.ogg")[..])).unwrap();
            while decoder.write_samples(&mut [0; 1000][..]) != 0 {}
        })
    });
    c.bench_function("wav decoder", |b| {
        b.iter(|| {
            let mut decoder =
                WavDecoder::new(Cursor::new(&include_bytes!("../examples/ilussion.wav")[..]))
                    .unwrap();
            while decoder.write_samples(&mut [0; 1000][..]) != 0 {}
        })
    });

    c.bench_function("mixer", |b| {
        b.iter(|| {
            let mut mixer = Mixer::new(1, audio_engine::SampleRate(44100));
            for _ in 0..1000 {
                let id = mixer.add_sound(
                    (),
                    criterion::black_box(Box::new(Nop) as Box<dyn SoundSource + Send>),
                );
                mixer.play(id);
            }
            mixer.write_samples(&mut [0; 1 << 15]);
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
