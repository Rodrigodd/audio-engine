use super::SoundSource;
use std::vec;

pub struct ChannelConverter<T: SoundSource> {
    inner: T,
    channels: u16,
}
impl<T: SoundSource> ChannelConverter<T> {
    pub fn new(inner: T, channels: u16) -> Self{
        Self {
            inner,
            channels
        }
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
            len*self.channels as usize
        } else if self.channels == 1 {
            let mut in_buffer = vec![0i16; buffer.len() * self.inner.channels() as usize];
            let len = self.inner.write_samples(&mut in_buffer);
            let mut sum: i32 = 0;
            for i in 0..len {
                sum += in_buffer[i] as i32;
                if (i + 1) % self.inner.channels() as usize == 0 {
                    buffer[i / self.inner.channels() as usize] = (sum / self.inner.channels() as i32) as i16;
                    sum = 0;
                }
            }
            len / self.inner.channels() as usize
        } else {
            unimplemented!("ChannelConventer only convert from 1 channel, or to 1 channel")
        }
    }
}


/// Do a sample rate convention using linear interpolation
pub struct SampleRateConverter<T: SoundSource> {
    inner: T,
    sample_rate: u32,
    in_buffer: Box<[i16]>,
    out_buffer: Box<[i16]>,
    iter: usize,
}
impl<T: SoundSource> SampleRateConverter<T> {
    pub fn new(inner: T, sample_rate: u32) -> Self {
        use gcd::Gcd;
        let gcd = inner.sample_rate().gcd(sample_rate) as usize;
        let out_buffer = vec![0; sample_rate as usize / gcd * inner.channels() as usize].into_boxed_slice();
        Self {
            in_buffer: vec![0; inner.sample_rate() as usize / gcd * inner.channels() as usize].into_boxed_slice(),
            iter: out_buffer.len(),
            out_buffer,
            inner,
            sample_rate
        }
    }

    // Return false if there is no more samples from inner
    fn convert_samples(&mut self) -> bool {
        let len = self.inner.write_samples(&mut self.in_buffer);
        if len == 0 {
            return false;
        }
        let channels = self.inner.channels() as usize;
        for c in 0..channels {
            self.out_buffer[c] = self.in_buffer[c];
            self.out_buffer[self.out_buffer.len() - channels + c] = self.in_buffer[self.in_buffer.len() - channels + c];
        }
        let mut i = 0;
        while i < self.out_buffer.len() * len / self.in_buffer.len() - channels {
            let j = (i*self.in_buffer.len()) as f32 / self.out_buffer.len() as f32 / channels as f32;
            let t = j.fract();
            let j = j as usize * channels;
            for c in 0..channels {
                self.out_buffer[i + c] = (self.in_buffer[j + c] as f32 * t + self.in_buffer[j + c + channels] as f32 * (1.0-t)) as i16;
            }
            i += channels;
        }
        self.iter = 0;
        true
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
        self.inner.reset()
    }
    fn write_samples(&mut self, buffer: &mut [i16]) -> usize {
        let mut i = 0;
        while i < buffer.len() {
            if self.iter < self.out_buffer.len() {
                buffer[i] = self.out_buffer[self.iter];
                i+=1;
                self.iter += 1;
            } else {
                if !self.convert_samples() {
                    return i;
                }
            }
        }

        buffer.len()
    }
}