use hound::WavReader;
use std::io::{Read, Seek};

use crate::SoundSource;

/// A SourceSource, from wav encoded sound data.
pub struct WavDecoder<T: Seek + Read + Send + 'static> {
    reader: WavReader<T>,
    channels: u16,
    sample_rate: u32,
}
impl<T: Seek + Read + Send + 'static> WavDecoder<T> {
    /// Create a new WavDecoder from the given .wav data.
    pub fn new(data: T) -> Result<Self, hound::Error> {
        let reader = WavReader::new(data)?;
        Ok(Self {
            channels: reader.spec().channels,
            sample_rate: reader.spec().sample_rate,
            reader,
        })
    }

    #[allow(clippy::needless_range_loop)]
    fn inner_write_sample<S: hound::Sample>(
        &mut self,
        buffer: &mut [i16],
        to_i16: impl Fn(S) -> i16,
    ) -> usize {
        let mut samples = self.reader.samples::<S>();
        for i in 0..buffer.len() {
            if let Some(sample) = samples.next() {
                buffer[i] = match sample {
                    Ok(x) => to_i16(x),
                    Err(err) => {
                        log::error!("error while decoding wav: {}", err);
                        // Returning the current number of decoded samples before the error,
                        // indicating that the SoundSource finished.
                        // FIXME: If this SoundSource was marked to loop, then this Error will
                        // repeat indefinitely. Maybe there should be a mechanism to report errors
                        // from a SoundSource.
                        return i;
                    }
                }
            } else {
                return i;
            }
        }
        buffer.len()
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
        let sample_format = self.reader.spec().sample_format;
        let bits_per_sample = self.reader.spec().bits_per_sample;
        match (sample_format, bits_per_sample) {
            (hound::SampleFormat::Float, _) => self.inner_write_sample(buffer, f32_to_i16),
            // 24bit or 32bit
            (hound::SampleFormat::Int, x) if x > 16 => {
                self.inner_write_sample(buffer, |x: i32| (x >> (bits_per_sample - 16)) as i16)
            }
            // 16bit
            (hound::SampleFormat::Int, x) if x == 16 => self.inner_write_sample(buffer, |x: i16| x),
            // 8bit
            (hound::SampleFormat::Int, _) => {
                self.inner_write_sample(buffer, |x: i8| (x as i16) << 8)
            }
        }
    }
}

fn f32_to_i16(mut x: f32) -> i16 {
    if x > 1.0 {
        x = 1.0
    }
    if x < -1.0 {
        x = -1.0
    }
    if x >= 0.0 {
        (x * i16::MAX as f32) as i16
    } else {
        (-x * i16::MIN as f32) as i16
    }
}
