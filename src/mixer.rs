use crate::{converter, SampleRate, SoundId, SoundSource};
use std::{
    collections::HashMap,
    hash::Hash,
    sync::atomic::{AtomicU64, Ordering},
};

fn next_id() -> SoundId {
    static GLOBAL_COUNT: AtomicU64 = AtomicU64::new(0);
    GLOBAL_COUNT.fetch_add(1, Ordering::Relaxed)
}

struct SoundInner<G = ()> {
    id: SoundId,
    data: Box<dyn SoundSource + Send>,
    volume: f32,
    group: G,
    looping: bool,
    drop: bool,
}
impl<G> SoundInner<G> {
    fn new(group: G, data: Box<dyn SoundSource + Send>) -> Self {
        Self {
            id: next_id(),
            data,
            volume: 1.0,
            group,
            looping: false,
            drop: true,
        }
    }
}

/// Keep track of each Sound, and mix they output together.
pub struct Mixer<G: Eq + Hash + Send + 'static = ()> {
    sounds: Vec<SoundInner<G>>,
    playing: usize,
    channels: u16,
    sample_rate: SampleRate,
    group_volumes: HashMap<G, f32>,
}

impl<G: Eq + Hash + Send + 'static> Mixer<G> {
    /// Create a new Mixer.
    ///
    /// The created Mixer output samples with given sample rate and number of channels. This
    /// configuration can be changed by calling [`set_config`](Self::set_config).
    pub fn new(channels: u16, sample_rate: SampleRate) -> Self {
        Self {
            sounds: vec![],
            playing: 0,
            channels,
            sample_rate,
            group_volumes: HashMap::new(),
        }
    }

    /// Change the number of channels and the sample rate.
    ///
    /// This keep also keep all currently playing sounds, and convert them to the new config, if
    /// necessary.
    pub fn set_config(&mut self, channels: u16, sample_rate: SampleRate) {
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

    /// Add new sound to the Mixer.
    ///
    /// Return the SoundId associated with that sound. This Id is globally unique.
    ///
    /// The added sound is started in stopped state, and [`play`](Self::play) must be called to start playing
    /// it. [`mark_to_remove`](Self::mark_to_remove) is true by default.
    pub fn add_sound(&mut self, group: G, sound: Box<dyn SoundSource + Send>) -> SoundId {
        let sound_inner = SoundInner::new(group, sound);
        let id = sound_inner.id;
        self.sounds.push(sound_inner);
        id
    }

    /// Start playing the sound associated with the given id.
    ///
    /// If the sound was paused or stop, it will start playing again.
    /// Otherwise, does nothing.
    pub fn play(&mut self, id: SoundId) {
        for i in (self.playing..self.sounds.len()).rev() {
            if self.sounds[i].id == id {
                self.sounds.swap(self.playing, i);
                self.playing += 1;
                break;
            }
        }
    }

    /// Pause the sound associated with the given id.
    ///
    /// If the sound is playing, it will pause. If play is called,
    /// this sound will continue from where it was when paused.
    /// If the sound is not playing, does nothing.
    pub fn pause(&mut self, id: SoundId) {
        for i in (0..self.playing).rev() {
            if self.sounds[i].id == id {
                self.playing -= 1;
                self.sounds.swap(self.playing, i);
                break;
            }
        }
    }

    /// Stop the sound associated with the given id.
    ///
    /// If the sound is playing, it will pause and reset the song. When play is called,
    /// this sound will start from the begging.
    ///
    /// Even if the sound is not playing, it will reset the sound to the start.
    pub fn stop(&mut self, id: SoundId) {
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

    /// Reset the sound associated with the given id.
    ///
    /// This reset the sound to the start, the sound being playing or not.
    pub fn reset(&mut self, id: SoundId) {
        for i in (0..self.sounds.len()).rev() {
            if self.sounds[i].id == id {
                self.sounds[i].data.reset();
                break;
            }
        }
    }

    /// Set the volume of the sound associated with the given id.
    ///
    /// The output samples of the SoundSource assicociated with the given id will be multiplied by
    /// this volume.
    pub fn set_volume(&mut self, id: SoundId, volume: f32) {
        for i in (0..self.sounds.len()).rev() {
            if self.sounds[i].id == id {
                self.sounds[i].volume = volume;
                break;
            }
        }
    }

    /// Set the volume of the given group.
    ///
    /// The volume of all sounds associated with this group is multiplied by this volume.
    pub fn set_group_volume(&mut self, group: G, volume: f32) {
        self.group_volumes.insert(group, volume);
    }

    /// Set if the sound associated with the given id will loop.
    ///
    /// If true, ever time the sound reachs its end, it will reset, and continue to play in a loop.
    ///
    /// This also set [`mark_to_remove`](Self::mark_to_remove) to false.
    pub fn set_loop(&mut self, id: SoundId, looping: bool) {
        for i in (0..self.sounds.len()).rev() {
            if self.sounds[i].id == id {
                self.sounds[i].looping = looping;
                break;
            }
        }
    }

    /// Mark if the sound will be removed after it reachs its end.
    ///
    /// If false, it will be possible to reset the sound and play it again after it has already
    /// reached its end. Otherwise, the sound will be removed when it reachs its end, even if it is
    /// marked to loop.
    pub fn mark_to_remove(&mut self, id: SoundId, drop: bool) {
        for i in (0..self.sounds.len()).rev() {
            if self.sounds[i].id == id {
                self.sounds[i].drop = drop;
                break;
            }
        }
    }
}

impl<G: Eq + Hash + Send + 'static> SoundSource for Mixer<G> {
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

            let group_volume = *self
                .group_volumes
                .get(&self.sounds[s].group)
                .unwrap_or(&1.0);
            let volume = self.sounds[s].volume * group_volume;

            if (volume - 1.0).abs() < 1.0 / i16::max_value() as f32 {
                for i in 0..len {
                    buffer[i] = buffer[i].saturating_add(buf[i]);
                }
            } else {
                for i in 0..len {
                    buffer[i] = buffer[i].saturating_add((buf[i] as f32 * volume) as i16);
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
