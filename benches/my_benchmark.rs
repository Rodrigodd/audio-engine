use audio_engine::{OggDecoder, SoundSource, WavDecoder};
use criterion::{criterion_group, criterion_main, Criterion};
use std::io::Cursor;

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
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
