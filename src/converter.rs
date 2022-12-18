//! Structs for converting SoundSource parameters, like number of channels and sample rate.

use super::SoundSource;
use std::vec;

#[cfg(test)]
mod test {
    use crate::SoundSource;

    use super::{ChannelConverter, SampleRateConverter};

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

    #[test]
    fn channels_1_3() {
        let inner = BufferSource {
            sample_rate: 30,
            channels: 1,
            buffer: vec![-2, -1, 0, 1, 2],
            i: 0,
        };

        let out_channels = 3;

        let mut output = vec![0; 3 * 5];
        let mut outer = ChannelConverter::new(inner, out_channels);

        outer.write_samples(&mut output);

        assert_eq!(output, [-2, -2, -2, -1, -1, -1, 0, 0, 0, 1, 1, 1, 2, 2, 2]);
    }

    #[test]
    fn channels_3_1() {
        let inner = BufferSource {
            sample_rate: 30,
            channels: 3,
            buffer: vec![1, 2, 3, 4, 5, 6, 7, 8, 9],
            i: 0,
        };

        let mut output = vec![0; 3];
        let mut outer = ChannelConverter::new(inner, 1);

        outer.write_samples(&mut output);

        assert_eq!(output, [2, 5, 8]);
    }

    #[test]
    fn channels_2_2() {
        let inner = BufferSource {
            sample_rate: 30,
            channels: 2,
            buffer: vec![1, 2, 3, 4, 5, 6, 7, 8, 9],
            i: 0,
        };

        let input = inner.buffer.clone();
        let mut output = vec![0; inner.buffer.len()];
        let mut outer = ChannelConverter::new(inner, 2);
        outer.write_samples(&mut output);
        assert_eq!(output, input);
    }

    #[test]
    fn channels_4_5() {
        let inner = BufferSource {
            sample_rate: 30,
            channels: 4,
            buffer: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
            i: 0,
        };
        let mut output = vec![0; inner.buffer.len() / 4 * 5];
        let mut outer = ChannelConverter::new(inner, 5);
        outer.write_samples(&mut output);
        assert_eq!(output, &[2, 2, 2, 2, 2, 6, 6, 6, 6, 6, 10, 10, 10, 10, 10,]);
    }

    #[test]
    fn channels_5_3() {
        let inner = BufferSource {
            sample_rate: 30,
            channels: 5,
            buffer: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
            i: 0,
        };
        let mut output = vec![0; inner.buffer.len() / 5 * 3];
        let mut outer = ChannelConverter::new(inner, 3);
        outer.write_samples(&mut output);
        assert_eq!(output, &[3, 3, 3, 8, 8, 8, 13, 13, 13]);

        let inner = BufferSource {
            sample_rate: 30,
            channels: 5,
            buffer: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
            i: 0,
        };
        let mut output = vec![0; 3];
        let mut outer = ChannelConverter::new(inner, 3);
        let len = outer.write_samples(&mut output);
        assert_eq!(output, &[3, 3, 3]);
        assert_eq!(len, 3);
        let len = outer.write_samples(&mut output);
        assert_eq!(output, &[8, 8, 8]);
        assert_eq!(len, 3);
        let len = outer.write_samples(&mut output);
        assert_eq!(output, &[13, 13, 13]);
        assert_eq!(len, 3);
        let len = outer.write_samples(&mut output);
        assert_eq!(&output[..len], &[]);
        assert_eq!(len, 0);
    }
}

/// Convert a SoundSource to a diferent number of channels.
///
/// If the number of channels in the inner SoundSource is equal to the output number of channels,
/// no conversion will be performed. Otherwise, each channel of the output will receive the average
/// of all input channels.
pub struct ChannelConverter<T: SoundSource> {
    inner: T,
    /// The number of channels to convert to.
    channels: u16,
    /// A buffer to temporary hold the input samples.
    in_buffer: Vec<i16>,
}
impl<T: SoundSource> ChannelConverter<T> {
    /// Create a new ChannelConverter.
    ///
    /// This will convert from the number of channels of `inner`, outputing the given number of
    /// `channels`.
    pub fn new(inner: T, channels: u16) -> Self {
        Self {
            inner,
            channels,
            in_buffer: Vec::new(),
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
    fn write_samples(&mut self, out_buffer: &mut [i16]) -> usize {
        let out_channels = self.channels as usize;
        let in_channels = self.inner.channels() as usize;

        use std::cmp::Ordering;
        match in_channels.cmp(&out_channels) {
            Ordering::Equal => self.inner.write_samples(out_buffer),
            Ordering::Less => {
                // To avoid a allocation, the input samples will be written to `out_buffer`, and
                // then converted to output samples.
                let in_len = out_buffer.len() / out_channels * in_channels;
                let in_len = self.inner.write_samples(&mut out_buffer[0..in_len]);

                let mut sum: i32 = 0;
                for i in (0..in_len).rev() {
                    sum += out_buffer[i] as i32;
                    if i % in_channels == 0 {
                        let frame_index = i / in_channels * out_channels;
                        let mean = (sum / in_channels as i32) as i16;
                        for c in 0..out_channels {
                            out_buffer[frame_index + c] = mean;
                        }
                        sum = 0;
                    }
                }
                in_len * out_channels / in_channels
            }
            Ordering::Greater => {
                // There are more input samples than output samples, so the allocation avoidance of
                // the previous arm does not work.
                let in_buffer = {
                    let len = out_buffer.len() / out_channels * in_channels;
                    if len > self.in_buffer.len() {
                        self.in_buffer.resize(len, 0);
                    }
                    &mut self.in_buffer[0..len]
                };
                let in_len = self.inner.write_samples(in_buffer);

                let mut sum: i32 = 0;
                for (i, &in_sample) in in_buffer[0..in_len].iter().enumerate() {
                    sum += in_sample as i32;
                    if (i + 1) % in_channels == 0 {
                        let frame_index = i / in_channels * out_channels;
                        let mean = (sum / in_channels as i32) as i16;
                        for c in 0..out_channels {
                            out_buffer[frame_index + c] = mean;
                        }
                        sum = 0;
                    }
                }
                in_len * out_channels / in_channels
            }
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
