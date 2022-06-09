//! Structs for converting SoundSource parameters, like number of channels and sample rate.

use super::SoundSource;
use std::vec;

#[cfg(test)]
mod test {
    use crate::SoundSource;

    use super::SampleRateConverter;

    struct BufferSource {
        sample_rate: u32,
        channels: u16,
        buffer: Vec<i16>,
        i: usize,
    }
    impl SoundSource for BufferSource {
        fn channels(&self) -> u16 {
            self.channels
        }

        fn sample_rate(&self) -> u32 {
            self.sample_rate
        }

        fn reset(&mut self) {
            self.i = 0;
        }

        fn write_samples(&mut self, buffer: &mut [i16]) -> usize {
            let i = self.i;
            let len = (self.buffer.len() - i).min(buffer.len());
            buffer[0..len].copy_from_slice(&self.buffer[i..i + len]);
            self.i += len;
            len
        }
    }

    #[test]
    fn sample_rate_1_3() {
        let inner = BufferSource {
            sample_rate: 10,
            channels: 1,
            buffer: vec![0, 3, 6, 9, 12],
            i: 0,
        };
        let mut outer = SampleRateConverter::new(inner, 30);

        let mut output = [0; 3];
        let len = outer.write_samples(&mut output[..]);

        assert_eq!(len, output.len());
        assert_eq!(output, [0, 1, 2]);

        let mut output = [0; 4];

        let len = outer.write_samples(&mut output[..]);
        assert_eq!(len, output.len());
        assert_eq!(output, [3, 4, 5, 6]);

        let len = outer.write_samples(&mut output[..]);
        assert_eq!(len, output.len());
        assert_eq!(output, [7, 8, 9, 10]);

        let len = outer.write_samples(&mut output[..]);
        assert_eq!(len, 2);
        assert_eq!(output[..len], [11, 12]);

        let len = outer.write_samples(&mut output[..]);
        assert_eq!(len, 0);
    }

    #[test]
    fn sample_rate_2_3() {
        let inner = BufferSource {
            sample_rate: 20,
            channels: 1,
            buffer: vec![0, 3, 6, 9, 12, 15],
            i: 0,
        };
        let mut outer = SampleRateConverter::new(inner, 30);

        let mut output = [0; 3];
        let len = outer.write_samples(&mut output[..]);

        assert_eq!(len, output.len());
        assert_eq!(output, [0, 2, 4]);

        let mut output = [0; 4];

        let len = outer.write_samples(&mut output[..]);
        assert_eq!(len, output.len());
        assert_eq!(output, [6, 8, 10, 12]);

        let len = outer.write_samples(&mut output[..]);
        assert_eq!(len, 1);
        assert_eq!(output[..len], [14]);

        let len = outer.write_samples(&mut output[..]);
        assert_eq!(len, 0);
    }

    #[test]
    fn sample_rate_3_2() {
        let inner = BufferSource {
            sample_rate: 30,
            channels: 1,
            buffer: vec![0, 2, 4, 6, 8, 10, 12, 14, 16, 18],
            i: 0,
        };
        let mut outer = SampleRateConverter::new(inner, 20);

        let mut output = [0; 2];
        let len = outer.write_samples(&mut output[..]);

        assert_eq!(len, output.len());
        assert_eq!(output, [0, 3]);

        let mut output = [0; 4];

        let len = outer.write_samples(&mut output[..]);
        assert_eq!(len, output.len());
        assert_eq!(output, [6, 9, 12, 15]);

        let len = outer.write_samples(&mut output[..]);
        assert_eq!(len, 1);
        assert_eq!(output[..len], [18]);

        let len = outer.write_samples(&mut output[..]);
        assert_eq!(len, 0);
    }
}

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
    /// The output sample_rate
    output_sample_rate: u32,
    /// a buffer contained a `in_len` of input samples, that will be completelly converted in
    /// `out_len` of ouput samples.
    in_buffer: Box<[i16]>,
    out_len: usize,
    /// The current length of valid samples in `in_buffer`.
    len: usize,
    /// The index of the next sample to be generated in the `out_buffer`. `out_buffer` don't exist
    /// in fact, and it samples are directly outputed in `write_samples`.
    iter: usize,
}
impl<T: SoundSource> SampleRateConverter<T> {
    /// Create a new SampleRateConverter.
    ///
    /// This will convert from the sample rate of `inner`, outputing with the given `sample_rate`.
    pub fn new(inner: T, output_sample_rate: u32) -> Self {
        use gcd::Gcd;

        // divide the input sample_rate and the ouput sample_rate by its gcd, to find to smallest
        // pair of input/output buffers that can be fully converted between.
        let gcd = inner.sample_rate().gcd(output_sample_rate) as usize;
        let in_len = inner.sample_rate() as usize / gcd * inner.channels() as usize;
        let out_len = output_sample_rate as usize / gcd * inner.channels() as usize;

        let channels = inner.channels() as usize;

        // in_buffer also contains the first sample of the next buffer.
        let in_buffer = vec![0; in_len + channels].into_boxed_slice();

        let mut this = Self {
            len: in_buffer.len() - 1,
            in_buffer,
            iter: out_len,
            out_len,
            inner,
            output_sample_rate,
        };

        this.reset();

        this
    }
}
impl<T: SoundSource> SoundSource for SampleRateConverter<T> {
    fn channels(&self) -> u16 {
        self.inner.channels()
    }
    fn sample_rate(&self) -> u32 {
        self.output_sample_rate
    }
    fn reset(&mut self) {
        self.inner.reset();

        let channels = self.inner.channels() as usize;
        self.len = self.inner.write_samples(&mut self.in_buffer[..]) - channels;
        self.iter = 0;
    }
    fn write_samples(&mut self, buffer: &mut [i16]) -> usize {
        let channels = self.inner.channels() as usize;

        if self.output_sample_rate == self.inner.sample_rate() {
            return self.inner.write_samples(buffer);
        }

        let mut i = 0;
        while i < buffer.len() {
            let in_len = self.in_buffer.len() - channels;
            fn div_up(a: usize, b: usize) -> usize {
                a / b + (a % b != 0) as usize
            }
            let curr_out_len = div_up(self.out_len * self.len, in_len) / channels * channels;

            // if next sample is out of bounds, reset in_buffer
            if self.iter >= curr_out_len {
                // if self.len is smaller than in_len, the inner sound already finished.
                if self.len < in_len {
                    return i;
                }

                // the last sample of the last buffer is the start sample of this buffer.
                self.in_buffer.copy_within(self.len.., 0);

                self.len = self.inner.write_samples(&mut self.in_buffer[channels..]);
                self.iter = 0;
            }

            // j is the float position in in_buffer.
            let j = ((self.iter / channels) * in_len) as f32 / self.out_len as f32;

            let t = j.fract();
            let j = j as usize * channels;

            for c in 0..channels {
                // interpolate by t, curr and next sample
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
