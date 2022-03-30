//! Structs for converting SoundSource paramters, like channel number and sample rate.

use super::SoundSource;
use std::vec;

/// Convert a SoundSource to a diferent number of channels.
///
/// This struct is able to convert from 1 channel to many (by duplicating the signal), or from many
/// channels to 1 (by averaging all channels). This panics for any other combination.
pub struct ChannelConverter<T: SoundSource> {
    inner: T,
    channels: u16,
}
impl<T: SoundSource> ChannelConverter<T> {
    /// Create a new ChannelConverter.
    ///
    /// This will convert from the number of channels of `inner`, outputing the given number of
    /// `channels`.
    pub fn new(inner: T, channels: u16) -> Self {
        Self { inner, channels }
    }
}
impl<T: SoundSource> SoundSource for ChannelConverter<T> {
    fn channels(&self) -> u16 {
        self.channels
    }
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }
    fn reset(&mut self) {
        self.inner.reset()
    }
    fn write_samples(&mut self, buffer: &mut [i16]) -> usize {
        if self.inner.channels() == 1 {
            let len = buffer.len() / self.channels as usize;
            let len = self.inner.write_samples(&mut buffer[0..len]);

            for i in (0..len).rev() {
                for c in 0..self.channels as usize {
                    buffer[i * self.channels as usize + c] = buffer[i];
                }
            }
            len * self.channels as usize
        } else if self.channels == 1 {
            let mut in_buffer = vec![0i16; buffer.len() * self.inner.channels() as usize];
            let len = self.inner.write_samples(&mut in_buffer);
            let mut sum: i32 = 0;
            for i in 0..len {
                sum += in_buffer[i] as i32;
                if (i + 1) % self.inner.channels() as usize == 0 {
                    buffer[i / self.inner.channels() as usize] =
                        (sum / self.inner.channels() as i32) as i16;
                    sum = 0;
                }
            }
            len / self.inner.channels() as usize
        } else {
            unimplemented!("ChannelConventer only convert from 1 channel, or to 1 channel")
        }
    }
}

/// Do a sample rate convertion using linear interpolation.
pub struct SampleRateConverter<T: SoundSource> {
    inner: T,
    sample_rate: u32,
    in_buffer: Box<[i16]>,
    out_len: usize,
    len: usize,
    iter: usize,
}
impl<T: SoundSource> SampleRateConverter<T> {
    /// Create a new SampleRateConverter.
    ///
    /// This will convert from the sample rate of `inner`, outputing with the given `sample_rate`.
    pub fn new(inner: T, sample_rate: u32) -> Self {
        use gcd::Gcd;
        let gcd = inner.sample_rate().gcd(sample_rate) as usize;
        let in_buffer = vec![0; inner.sample_rate() as usize / gcd * inner.channels() as usize]
            .into_boxed_slice();
        let out_len = sample_rate as usize / gcd * inner.channels() as usize;
        Self {
            len: in_buffer.len(),
            in_buffer,
            iter: out_len,
            out_len,
            inner,
            sample_rate,
        }
    }
}
impl<T: SoundSource> SoundSource for SampleRateConverter<T> {
    fn channels(&self) -> u16 {
        self.inner.channels()
    }
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn reset(&mut self) {
        self.inner.reset();
        self.iter = self.out_len;
        self.len = self.in_buffer.len();
    }
    fn write_samples(&mut self, buffer: &mut [i16]) -> usize {
        let mut i = 0;
        let channels = self.inner.channels() as usize;
        if self.in_buffer.len() == channels {
            return self.inner.write_samples(buffer);
        }
        while i < buffer.len() {
            if self.iter + channels >= self.out_len * self.len / self.in_buffer.len() {
                if self.len < self.in_buffer.len() {
                    return i;
                }
                self.len = self.inner.write_samples(&mut self.in_buffer);
                self.iter = 0;
            }
            let j = ((self.iter / channels) * self.in_buffer.len()) as f32 / self.out_len as f32;
            let t = j.fract();
            let j = j as usize * channels;
            for c in 0..channels {
                buffer[i + c] = (self.in_buffer[j + c] as f32 * (1.0 - t)
                    + self.in_buffer[j + c + channels] as f32 * t)
                    as i16;
            }
            self.iter += channels;
            i += channels;
        }

        buffer.len()
    }
}
