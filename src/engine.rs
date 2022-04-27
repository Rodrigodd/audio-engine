use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use super::{Mixer, Sound, SoundSource};
use crate::converter::{ChannelConverter, SampleRateConverter};

/// The main struct of the crate.
///
/// This hold all existing `SoundSource`s and `cpal::platform::Stream`.
pub struct AudioEngine {
    mixer: Arc<Mutex<Mixer>>,
    channels: u16,
    sample_rate: u32,
    _stream: cpal::platform::Stream,
}
impl AudioEngine {
    /// Tries to create a new AudioEngine.
    ///
    /// `cpal` will spawn a new thread where the sound samples will be sampled, mixed, and outputed
    /// to the output stream.
    pub fn new() -> Result<Self, &'static str> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("no output device available")?;
        let mut supported_configs_range = device
            .supported_output_configs()
            .map_err(|_| "error while querying formats")?;
        let config = supported_configs_range
            .next()
            .ok_or("no supported format?!")?
            .with_max_sample_rate();

        let sample_format = config.sample_format();
        let config = config.config();

        let mixer = Arc::new(Mutex::new(Mixer::new(
            config.channels,
            super::SampleRate(config.sample_rate.0),
        )));

        let stream = {
            match sample_format {
                cpal::SampleFormat::I16 => stream::<i16>(&mixer, device, config),
                cpal::SampleFormat::U16 => stream::<i16>(&mixer, device, config),
                cpal::SampleFormat::F32 => stream::<f32>(&mixer, device, config),
            }
        };
        stream.play().unwrap();

        let m = mixer.lock().unwrap();
        let channels = m.channels;
        let sample_rate = m.sample_rate;
        drop(m);
        Ok(Self {
            mixer,
            channels,
            sample_rate,
            _stream: stream,
        })
    }

    /// The sample rate that is currently being outputed to the device.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// The number of channels that is currently being outputed to the device.
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Create a new Sound.
    ///
    /// Return a `Err` if the number of channels doesn't match the output number of channels. If
    /// the ouput number of channels is 1, or the number of channels of `source` is 1, `source`
    /// will be automatic wrapped in a [`ChannelConverter`].
    ///
    /// If the `sample_rate` of `source` mismatch the output `sample_rate`, `source` will be
    /// wrapped in a [`SampleRateConverter`].
    pub fn new_sound<T: SoundSource + Send + 'static>(
        &self,
        source: T,
    ) -> Result<Sound, &'static str> {
        let sound: Box<dyn SoundSource + Send> = if source.sample_rate() != self.sample_rate {
            if source.channels() == self.channels {
                Box::new(SampleRateConverter::new(source, self.sample_rate))
            } else if self.channels == 1 {
                Box::new(ChannelConverter::new(
                    SampleRateConverter::new(source, self.sample_rate),
                    self.channels,
                ))
            } else if source.channels() == 1 {
                Box::new(ChannelConverter::new(
                    SampleRateConverter::new(source, self.sample_rate),
                    self.channels,
                ))
            } else {
                return Err("Number of channels do not match the output, and is not 1");
            }
        } else if source.channels() == self.channels {
            Box::new(source)
        } else if self.channels == 1 {
            Box::new(ChannelConverter::new(source, self.channels))
        } else if source.channels() == 1 {
            Box::new(ChannelConverter::new(source, self.channels))
        } else {
            return Err("Number of channels do not match the output, and is not 1");
        };

        let id = self.mixer.lock().unwrap().add_sound(sound);
        Ok(Sound {
            mixer: self.mixer.clone(),
            id,
        })
    }
}

fn stream<T: cpal::Sample>(
    mixer: &Arc<Mutex<Mixer>>,
    device: cpal::Device,
    config: cpal::StreamConfig,
) -> cpal::Stream {
    let mixer = mixer.clone();
    device
        .build_output_stream(
            &config,
            move |buffer: &mut [T], _| {
                let mut buf = vec![0; buffer.len()];
                mixer.lock().unwrap().write_samples(&mut buf);
                for i in 0..buffer.len() {
                    buffer[i] = T::from(&buf[i]);
                }
            },
            move |err| match err {
                cpal::StreamError::DeviceNotAvailable => {
                    todo!("handle device disconnection")
                }
                cpal::StreamError::BackendSpecific { err } => panic!("{}", err),
            },
        )
        .unwrap()
}