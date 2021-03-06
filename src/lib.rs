#![warn(missing_docs)]

//! Audio Engine is a cross-platform crate for audio playback, build on top of cpal.
//!
//! ## Supported formats
//! - ogg
//! - wav
//!
//! ## Example
//!
//! ```no_run
//! # fn main() -> Result<(), &'static str> {
//! # let my_wav_sound = std::io::Cursor::new(vec![]);
//! use audio_engine::{AudioEngine, WavDecoder};
//! let audio_engine = AudioEngine::new()?;
//! let mut sound = audio_engine.new_sound(WavDecoder::new(my_wav_sound))?;
//! sound.play();
//! # Ok(())
//! # }
//! ```

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};

pub mod converter;
mod ogg;
mod wav;

mod engine;
pub use engine::AudioEngine;

pub use ogg::OggDecoder;
pub use wav::WavDecoder;

type SoundId = u64;

fn next_id() -> SoundId {
    static GLOBAL_COUNT: AtomicU64 = AtomicU64::new(0);
    GLOBAL_COUNT.fetch_add(1, Ordering::Relaxed)
}

/// The number of samples processed per second for a single channel of audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SampleRate(pub u32);

/// Represents a sound in the AudioEngine. If this is dropped, the sound will continue to play
/// until it ends.
pub struct Sound {
    mixer: Arc<Mutex<Mixer>>,
    id: SoundId,
}
impl Sound {
    /// Starts or continue to play the sound.
    ///
    /// If the sound was paused or stop, it will start playing again.
    /// Otherwise, does nothing.
    pub fn play(&mut self) {
        self.mixer.lock().unwrap().play(self.id);
    }

    /// Pause the sound.
    ///
    /// If the sound is playing, it will pause. If play is called,
    /// this sound will continue from where it was before pause.
    /// If the sound is not playing, does nothing.
    pub fn pause(&mut self) {
        self.mixer.lock().unwrap().pause(self.id);
    }

    /// Stop the sound.
    ///
    /// If the sound is playing, it will pause and reset the song. When play is called,
    /// this sound will start from the begging.
    /// Even if the sound is not playing, it will reset the sound to the start.
    pub fn stop(&mut self) {
        self.mixer.lock().unwrap().stop(self.id);
    }

    /// Reset the sound to the start.
    ///
    /// The behaviour is the same being the sound playing or not.
    pub fn reset(&mut self) {
        self.mixer.lock().unwrap().reset(self.id);
    }

    /// Set the volume of the sound.
    pub fn set_volume(&mut self, volume: f32) {
        self.mixer.lock().unwrap().set_volume(self.id, volume);
    }

    /// Set if the sound will repeat even time it reach the end.
    pub fn set_loop(&mut self, looping: bool) {
        self.mixer.lock().unwrap().set_loop(self.id, looping);
    }
}
impl Drop for Sound {
    fn drop(&mut self) {
        self.mixer.lock().unwrap().drop_sound(self.id);
    }
}

/// A source of sound samples.
///
/// Sound samples of each channel must be interleaved.
pub trait SoundSource {
    /// Return the number of channels.
    fn channels(&self) -> u16;

    /// Return the sample rate.
    fn sample_rate(&self) -> u32;

    /// Start the sound from the begining.
    fn reset(&mut self);

    /// Write the samples to `buffer`.
    ///
    /// Return how many samples was written. If it return a value less thand the length of
    /// `buffer`, this indicate that the sound ended.
    ///
    /// The `buffer` length and the returned length should always be a multiple of
    /// [`self.channels()`](SoundSource::channels).
    fn write_samples(&mut self, buffer: &mut [i16]) -> usize;
}
impl<T: SoundSource + ?Sized> SoundSource for Box<T> {
    fn channels(&self) -> u16 {
        (**self).channels()
    }

    fn sample_rate(&self) -> u32 {
        (**self).sample_rate()
    }

    fn reset(&mut self) {
        (**self).reset()
    }

    fn write_samples(&mut self, buffer: &mut [i16]) -> usize {
        (**self).write_samples(buffer)
    }
}

struct SoundInner {
    id: SoundId,
    data: Box<dyn SoundSource + Send>,
    volume: f32,
    looping: bool,
    drop: bool,
}
impl SoundInner {
    fn new(data: Box<dyn SoundSource + Send>) -> Self {
        Self {
            id: next_id(),
            data,
            volume: 1.0,
            looping: false,
            drop: false,
        }
    }
}

/// Keep track of each Sound, and mix they output together.
struct Mixer {
    sounds: Vec<SoundInner>,
    playing: usize,
    channels: u16,
    sample_rate: SampleRate,
}
impl Mixer {
    fn new(channels: u16, sample_rate: SampleRate) -> Self {
        Self {
            sounds: vec![],
            playing: 0,
            channels,
            sample_rate,
        }
    }

    /// Change the number of channels and the sample rate.
    ///
    /// This keep also keep all currently playing sounds, and convert them to the new config, if
    /// necessary.
    fn set_config(&mut self, channels: u16, sample_rate: SampleRate) {
        struct Nop;
        #[rustfmt::skip]
        impl SoundSource for Nop {
            fn channels(&self) -> u16 { 0 }
            fn sample_rate(&self) -> u32 { 0 }
            fn reset(&mut self) { }
            fn write_samples(&mut self, _: &mut [i16]) -> usize { 0 }
        }

        let not_chaged = self.channels == channels && self.sample_rate == sample_rate;
        if not_chaged {
            return;
        }
        if !self.sounds.is_empty() {
            for sound in self.sounds.iter_mut() {
                // FIXME: if the config change multiple times, this will nest multiple converts,
                // increasing processing and loosing quality.
                // Maybe I should create something like a tree of converters, and always keep the
                // convertes Concrete.
                if sound.data.channels() != channels {
                    let inner = std::mem::replace(&mut sound.data, Box::new(Nop));
                    sound.data = Box::new(converter::ChannelConverter::new(inner, channels));
                }
                if sound.data.sample_rate() != sample_rate.0 {
                    let inner = std::mem::replace(&mut sound.data, Box::new(Nop));
                    sound.data =
                        Box::new(converter::SampleRateConverter::new(inner, sample_rate.0));
                }
            }
        }
        self.channels = channels;
        self.sample_rate = sample_rate;
    }

    fn add_sound(&mut self, sound: Box<dyn SoundSource + Send>) -> SoundId {
        let sound_inner = SoundInner::new(sound);
        let id = sound_inner.id;
        self.sounds.push(sound_inner);
        id
    }

    /// If the sound was paused or stop, it will start playing again.
    /// Otherwise, does nothing.
    fn play(&mut self, id: SoundId) {
        for i in (self.playing..self.sounds.len()).rev() {
            if self.sounds[i].id == id {
                self.sounds.swap(self.playing, i);
                self.playing += 1;
                break;
            }
        }
    }

    /// If the sound is playing, it will pause. If play is called,
    /// this sound will continue from where it was when paused.
    /// If the sound is not playing, does nothing.
    fn pause(&mut self, id: SoundId) {
        for i in (0..self.playing).rev() {
            if self.sounds[i].id == id {
                self.playing -= 1;
                self.sounds.swap(self.playing, i);
                break;
            }
        }
    }

    /// If the sound is playing, it will pause and reset the song. When play is called,
    /// this sound will start from the begging.
    /// Even if the sound is not playing, it will reset the sound to the start.
    fn stop(&mut self, id: SoundId) {
        for i in (0..self.sounds.len()).rev() {
            if self.sounds[i].id == id {
                self.sounds[i].data.reset();
                if i < self.playing {
                    self.playing -= 1;
                    self.sounds.swap(self.playing, i);
                }
                break;
            }
        }
    }

    /// This reset the sound to the start, the sound being playing or not.
    fn reset(&mut self, id: SoundId) {
        for i in (0..self.sounds.len()).rev() {
            if self.sounds[i].id == id {
                self.sounds[i].data.reset();
                break;
            }
        }
    }

    /// Set the volume of the sound.
    fn set_volume(&mut self, id: SoundId, volume: f32) {
        for i in (0..self.sounds.len()).rev() {
            if self.sounds[i].id == id {
                self.sounds[i].volume = volume;
                break;
            }
        }
    }

    /// Set if the sound will repeat ever time it reach the end.
    fn set_loop(&mut self, id: SoundId, looping: bool) {
        for i in (0..self.sounds.len()).rev() {
            if self.sounds[i].id == id {
                self.sounds[i].looping = looping;
                break;
            }
        }
    }

    /// Mark the sound to be dropped after it reach the end.
    fn drop_sound(&mut self, id: SoundId) {
        for i in (0..self.sounds.len()).rev() {
            if self.sounds[i].id == id {
                self.sounds[i].drop = true;
                break;
            }
        }
    }
}
impl SoundSource for Mixer {
    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate.0
    }

    fn reset(&mut self) {}

    fn write_samples(&mut self, buffer: &mut [i16]) -> usize {
        if self.playing == 0 {
            for b in buffer.iter_mut() {
                *b = 0;
            }
            return buffer.len();
        }

        let mut buf = vec![0; buffer.len()];
        let mut s = 0;
        while s < self.playing {
            let mut len = 0;
            loop {
                len += self.sounds[s].data.write_samples(&mut buf[len..]);
                if len < buffer.len() {
                    self.sounds[s].data.reset();
                    if self.sounds[s].looping {
                        continue;
                    }
                }
                break;
            }

            if (self.sounds[s].volume - 1.0).abs() < 1.0 / i16::max_value() as f32 {
                for i in 0..len {
                    buffer[i] = buffer[i].saturating_add(buf[i]);
                }
            } else {
                for i in 0..len {
                    buffer[i] =
                        buffer[i].saturating_add((buf[i] as f32 * self.sounds[s].volume) as i16);
                }
            }

            if len < buffer.len() {
                if self.sounds[s].drop {
                    let _ = self.sounds.swap_remove(s);
                }
                self.playing -= 1;
                if self.playing > 0 && self.playing < self.sounds.len() {
                    self.sounds.swap(s, self.playing);
                } else {
                    break;
                }
            } else {
                s += 1;
            }
        }

        buffer.len()
    }
}
