use criterion::{black_box, criterion_group, criterion_main, Criterion};
use audio_engine::{ OggDecoder, WavDecoder, SoundSource };
use std::io::Cursor;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("ogg decoder", |b| b.iter(|| {
        let mut decoder = OggDecoder::new(Cursor::new(&include_bytes!("../examples/pipe.ogg")[..]));
        while decoder.write_samples(&mut [0; 1000][..]) < 1000 {};
    }));
    c.bench_function("wav decoder", |b| b.iter(|| {
        let mut decoder = WavDecoder::new(Cursor::new(&include_bytes!("../examples/ilussion.wav")[..]));
        while decoder.write_samples(&mut [0; 1000][..]) < 1000 {};
    }));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
