//! This contains a version of AudioEngine that don't depends on cpal, and uses the native web
//! AudioContext. This is single thread and must be frequentily updated.
//!
//! I made this because cpal doesn't supported wasm, but I think this has change. Also I don't knew
//! of Web Workers when I write this, so I don't took them in account.

use std::sync::{Arc, Mutex};

use super::{Mixer, Sound, SoundSource};
use crate::converter::{ChannelConverter, SampleRateConverter};

/// The main struct of the crate.
pub struct AudioEngine {
    ctx: web_sys::AudioContext,
    mixer: Arc<Mutex<Mixer>>,
    sample_rate: u32,
    channels: u16,
    next_time: f64,
}
impl AudioEngine {
    /// Tries to create a new AudioEngine.
    pub fn new() -> Result<Self, &'static str> {
        let ctx = web_sys::AudioContext::new().map_err(|_| "Failed to create AudioContext")?;
        let sample_rate = ctx.sample_rate() as u32;

        Ok(Self {
            sample_rate,
            channels: 2,
            next_time: ctx.current_time() + 0.01,
            ctx,
            mixer: Arc::new(Mutex::new(Mixer::new(2, super::SampleRate(sample_rate)))),
        })
    }

    /// Call this every 20ms to keep outputing sound correctly.
    ///
    /// This genrate 20ms of audio per call, and buffers up to 50 ms.
    pub fn update(&mut self) {
        let curr_time = self.ctx.current_time();
        if self.next_time > curr_time + 0.05 {
            return;
        }
        if self.next_time < curr_time {
            self.next_time = curr_time;
        }
        let length = self.sample_rate / 50;
        let audio_buffer = self
            .ctx
            .create_buffer(self.channels as u32, length, self.sample_rate as f32)
            .unwrap();
        let mut buffer = vec![0; length as usize * self.channels as usize];
        self.mixer.lock().unwrap().write_samples(&mut buffer);
        for i in 0..self.channels as usize {
            let mut channel = vec![0.0; length as usize];
            let mut b = i;
            let mut c = 0;
            while c < length as usize {
                channel[c] = buffer[b] as f32 / i16::max_value() as f32;
                b += self.channels as usize;
                c += 1;
            }
            audio_buffer.copy_to_channel(&mut channel, i as i32);
        }

        let source = self.ctx.create_buffer_source().unwrap();
        source.set_buffer(Some(&audio_buffer));
        source
            .connect_with_audio_node(&self.ctx.destination())
            .unwrap();
        source.start_with_when(self.next_time);
        self.next_time += 0.02;
    }

    /// The sample rate that is currently being output to the device.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Create a new Sound.
    pub fn new_sound<T: SoundSource + Send + 'static>(
        &self,
        source: T,
    ) -> Result<Sound, &'static str> {
        self.ctx.resume().unwrap();
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
