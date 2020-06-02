use std::io::{ Seek, Read };
use hound::WavReader;

use crate::SoundSource;

pub struct WavDecoder<T: Seek + Read + Send + 'static> {
    reader: WavReader<T>,
    channels: u16,
    sample_rate: u32,
}
impl<T: Seek + Read + Send + 'static> WavDecoder<T> {
    pub fn new(data: T) -> Self {
        let reader = WavReader::new(data).unwrap();
        Self {
            channels: reader.spec().channels,
            sample_rate: reader.spec().sample_rate,
            reader,
        }
    }
}
impl<T: Seek + Read + Send + 'static> SoundSource for WavDecoder<T> {
    fn reset(&mut self) {
        self.reader.seek(0).unwrap();
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn write_samples(&mut self, buffer: &mut [i16]) -> usize {
        let mut samples = self.reader.samples::<i16>();
        for i in 0..buffer.len() {
            if let Some(sample) = samples.next() {
                buffer[i] = sample.unwrap();
            } else {
                return i;
            }
        }

        buffer.len()
    }
}