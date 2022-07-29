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
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let my_wav_sound = std::io::Cursor::new(vec![]);
//! use audio_engine::{AudioEngine, WavDecoder};
//! let audio_engine = AudioEngine::new()?;
//! let mut sound = audio_engine.new_sound(WavDecoder::new(my_wav_sound)?)?;
//! sound.play();
//! # Ok(())
//! # }
//! ```

use std::{
    hash::Hash,
    sync::{Arc, Mutex},
};

pub mod converter;
mod ogg;
mod sine;
mod wav;

mod engine;
pub use engine::AudioEngine;

mod mixer;
pub use mixer::Mixer;

pub use ogg::OggDecoder;
pub use sine::SineWave;
pub use wav::WavDecoder;

/// The number of samples processed per second for a single channel of audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SampleRate(pub u32);

type SoundId = u64;

/// Represents a sound in the AudioEngine.
///
/// If this is dropped, the sound will continue to play, but will be removed
/// when it reachs its ends, even if it is set to loop.
pub struct Sound<G: Eq + Hash + Send + 'static = ()> {
    mixer: Arc<Mutex<Mixer<G>>>,
    id: SoundId,
}
impl<G: Eq + Hash + Send + 'static> Sound<G> {
    /// Starts or continue to play the sound.
    ///
    /// If the sound was paused or stop, it will start playing again. Otherwise,
    /// does nothing.
    pub fn play(&mut self) {
        self.mixer.lock().unwrap().play(self.id);
    }

    /// Pause the sound.
    ///
    /// If the sound is playing, it will pause. If play is called, this sound
    /// will continue from where it was before pause. If the sound is not
    /// playing, does nothing.
    pub fn pause(&mut self) {
        self.mixer.lock().unwrap().pause(self.id);
    }

    /// Stop the sound.
    ///
    /// If the sound is playing, it will pause and reset the song. When play is
    /// called, this sound will start from the beginning. Even if the sound is not
    /// playing, it will reset the sound to the start.
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

    /// Set if the sound will repeat ever time it reachs its end.
    pub fn set_loop(&mut self, looping: bool) {
        self.mixer.lock().unwrap().set_loop(self.id, looping);
    }
}
impl<G: Eq + Hash + Send + 'static> Drop for Sound<G> {
    fn drop(&mut self) {
        self.mixer.lock().unwrap().mark_to_remove(self.id, true);
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
impl<T: SoundSource + ?Sized> SoundSource for Arc<Mutex<T>> {
    fn channels(&self) -> u16 {
        (*self).lock().unwrap().channels()
    }

    fn sample_rate(&self) -> u32 {
        (*self).lock().unwrap().sample_rate()
    }

    fn reset(&mut self) {
        (*self).lock().unwrap().reset()
    }

    fn write_samples(&mut self, buffer: &mut [i16]) -> usize {
        (*self).lock().unwrap().write_samples(buffer)
    }
}
