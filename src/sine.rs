use std::f64::consts::TAU;

use crate::SoundSource;

/// A SoundSource that generates a sine wave with a given frequency.
pub struct SineWave {
    // With a sample_rte of 96000 Hz, this u64 variable will wrap after 6 million years.
    i: u64,
    /// The sample_rate of this SoundSource.
    pub sample_rate: u32,
    /// The frequency of the sine wave, in Hertz
    pub freq: f32,
}
impl SineWave {
    /// Create a new SineWave SoundSource.
    ///
    /// Created one the given sample_rate and frequency, both in Hertz.
    pub fn new(sample_rate: u32, freq: f32) -> Self {
        Self {
            i: 0,
            sample_rate,
            freq,
        }
    }
}
impl SoundSource for SineWave {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn channels(&self) -> u16 {
        1
    }
    fn reset(&mut self) {
        self.i = 0
    }
    fn write_samples(&mut self, out: &mut [i16]) -> usize {
        for o in out.iter_mut() {
            // With a mantissa of 52 bits, at 96000 Hz, i as f64 will lose precision after 1486
            // years.
            let t = self.i as f64 / self.sample_rate() as f64;
            let amplitude = (i16::max_value() / 4) as f64;
            *o = ((self.freq as f64 * TAU * t).cos() * amplitude) as i16;
            self.i += 1;
        }
        out.len()
    }
}
