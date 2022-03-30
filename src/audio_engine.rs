use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use super::{Mixer, Sound, SoundSource};
use crate::converter::{ChannelConverter, SampleRateConverter};

pub struct AudioEngine {
    mixer: Arc<Mutex<Mixer>>,
    channels: u16,
    sample_rate: u32,
    _stream: cpal::platform::Stream,
}
impl AudioEngine {
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
            .with_max_sample_rate()
            .config();

        let mixer = Arc::new(Mutex::new(Mixer::new(config.channels, config.sample_rate)));

        let stream = {
            let mixer = mixer.clone();
            device
                .build_output_stream(
                    &config,
                    move |buffer, _| {
                        let mut buf = vec![0; buffer.len()];
                        mixer.lock().unwrap().write_samples(&mut buf);
                        for i in 0..buffer.len() {
                            buffer[i] = buf[i] as f32 / i16::max_value() as f32;
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

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn new_sound<T: SoundSource + Send + 'static>(
        &self,
        source: T,
    ) -> Result<Sound, &'static str> {
        let sound: Box<dyn SoundSource + Send> = if source.sample_rate() != self.sample_rate {
            if source.channels() == self.channels {
                Box::new(SampleRateConverter::new(source, self.sample_rate))
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
