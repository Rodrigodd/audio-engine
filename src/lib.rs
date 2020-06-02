
use std::sync::{ Arc, Mutex, atomic::{ AtomicU64, Ordering } };

mod ogg;
mod wav;
mod converter;

#[cfg(not(target_arch = "wasm32"))]
mod audio_engine;

#[cfg(target_arch = "wasm32")]
mod web_audio_engine;

pub use ogg::OggDecoder;
pub use wav::WavDecoder;

#[cfg(not(target_arch = "wasm32"))]
pub use audio_engine::AudioEngine;

#[cfg(target_arch = "wasm32")]
pub use web_audio_engine::AudioEngine;

type SoundId = u64;

fn next_id() -> SoundId {
    static GLOBAL_COUNT: AtomicU64 = AtomicU64::new(0);
    GLOBAL_COUNT.fetch_add(1, Ordering::Relaxed)
}

pub struct Sound {
    mixer: Arc<Mutex<Mixer>>,
    id: SoundId,
}
impl Sound {
    pub fn play(&mut self) {
        self.mixer.lock().unwrap().play(self.id);
    }
    pub fn pause(&mut self) {
        self.mixer.lock().unwrap().pause(self.id);
    }
    pub fn stop(&mut self) {
        self.mixer.lock().unwrap().stop(self.id);
    }
    pub fn reset(&mut self) {
        self.mixer.lock().unwrap().reset(self.id);
    }
    pub fn set_volume(&mut self, volume: f32) {
        self.mixer.lock().unwrap().set_volume(self.id, volume);
    }

    pub fn set_loop(&mut self, looping: bool) {
        self.mixer.lock().unwrap().set_loop(self.id, looping);
    }
}
impl Drop for Sound {
    fn drop(&mut self) {
        self.mixer.lock().unwrap().drop_sound(self.id);
    }
}

pub trait SoundSource {
    /// Return the number of channels
    fn channels(&self) -> u16;

    /// return the sample rate
    fn sample_rate(&self) -> u32;

    /// Start the sound from the begining
    fn reset(&mut self);

    /// Write the samples to the buffer. Return how much has write.
    /// If it return a value less thand the length of the buffer,
    /// this indicate that the sound end.
    fn write_samples(&mut self, buffer: &mut [i16]) -> usize;
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

struct Mixer {
    sounds: Vec<SoundInner>,
    playing: usize,
    channels: u16,
    sample_rate: u32,
}
impl Mixer {
    fn new(channels: u16, sample_rate: u32) -> Self {
        Self {
            sounds: vec![],
            playing: 0,
            channels,
            sample_rate,
        }
    }
    fn add_sound(&mut self, sound: Box<dyn SoundSource + Send>) -> SoundId {
        let sound_inner = SoundInner::new(sound);
        let id = sound_inner.id;
        self.sounds.push(sound_inner);
        id
    }

    fn play(&mut self, id: SoundId) {
        for i in (self.playing..self.sounds.len()).rev() {
            if self.sounds[i].id == id {
                self.sounds.swap(self.playing, i);
                self.playing += 1;
                break;
            }
        }
    }

    fn pause(&mut self, id: SoundId) {
        for i in (0..self.playing).rev() {
            if self.sounds[i].id == id {
                self.playing -= 1;
                self.sounds.swap(self.playing, i);
                break;
            }
        }
    }

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

    fn reset(&mut self, id: SoundId) {
        for i in (0..self.sounds.len()).rev() {
            if self.sounds[i].id == id {
                // self.playing -= 1;
                // self.sounds.swap(self.playing, i);
                self.sounds[i].data.reset();
                break;
            }
        }
    }

    fn set_volume(&mut self, id: SoundId, volume: f32) {
        for i in (0..self.sounds.len()).rev() {
            if self.sounds[i].id == id {
                self.sounds[i].volume = volume;
                break;
            }
        }
    }

    fn set_loop(&mut self, id: SoundId, looping: bool) {
        for i in (0..self.sounds.len()).rev() {
            if self.sounds[i].id == id {
                self.sounds[i].looping = looping;
                break;
            }
        }
    }

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
        self.sample_rate
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
        let mut i = 0;
        while i < self.playing {
            let mut len = 0;
            loop {
                len += self.sounds[i].data.write_samples(&mut buf[len..]);
                if len < buffer.len() {
                    if self.sounds[i].looping {
                        self.sounds[i].data.reset();
                        continue;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            if self.sounds[0].volume == 1.0 {
                for i in 0..len {
                    buffer[i] = buffer[i].saturating_add(buf[i]);
                }
            } else {
                for i in 0..len {
                    buffer[i] = buffer[i].saturating_add((buf[i] as f32 * self.sounds[0].volume) as i16);
                }
            }

            if len < buffer.len() {
                if self.sounds[i].drop {
                    let _ = self.sounds.swap_remove(i);
                }
                self.playing -= 1;
                if self.playing > 0 && self.playing < self.sounds.len() {
                    self.sounds.swap(i, self.playing);
                } else {
                    break;
                }
            } else {
                i += 1;
            }
        }

        buffer.len()
    }
}
