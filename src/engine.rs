use std::{
    hash::Hash,
    sync::{Arc, Mutex},
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleRate, StreamError,
};

use super::{Mixer, Sound, SoundSource};
use crate::converter::{ChannelConverter, SampleRateConverter};

use backend::Backend;

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use super::create_device;
    use crate::Mixer;
    use std::{
        hash::Hash,
        sync::{Arc, Mutex},
    };

    struct StreamEventLoop<G: Eq + Hash + Send + 'static> {
        mixer: Arc<Mutex<Mixer<G>>>,
        stream: Option<cpal::platform::Stream>,
    }

    impl<G: Eq + Hash + Send + 'static> StreamEventLoop<G> {
        fn run(
            &mut self,
            event_channel: std::sync::mpsc::Sender<StreamEvent>,
            stream_event_receiver: std::sync::mpsc::Receiver<StreamEvent>,
        ) {
            // Trigger first device creation
            event_channel.send(StreamEvent::RecreateStream).unwrap();

            let mut handled = false;
            let error_callback = move |err| {
                log::error!("stream error: {}", err);
                if !handled {
                    // The Stream could have send multiple errors. I confirmed this happening on
                    // android (a error before the stream close, and a error after closing it).
                    handled = true;
                    event_channel.send(StreamEvent::RecreateStream).unwrap()
                }
            };

            while let Ok(event) = stream_event_receiver.recv() {
                match event {
                    StreamEvent::RecreateStream => {
                        log::debug!("recreating audio device");

                        // Droping the stream is unsound in android, see:
                        // https://github.com/katyo/oboe-rs/issues/41
                        #[cfg(target_os = "android")]
                        std::mem::forget(self.stream.take());

                        #[cfg(not(target_os = "android"))]
                        drop(self.stream.take());

                        let stream = create_device(&self.mixer, error_callback.clone());
                        let stream = match stream {
                            Ok(x) => x,
                            Err(x) => {
                                log::error!("creating audio device failed: {}", x);
                                return;
                            }
                        };
                        self.stream = Some(stream);
                    }
                    StreamEvent::Drop => {
                        // Droping the stream is unsound in android, see:
                        // https://github.com/katyo/oboe-rs/issues/41
                        #[cfg(target_os = "android")]
                        std::mem::forget(self.stream.take());

                        return;
                    }
                }
            }
        }
    }

    enum StreamEvent {
        RecreateStream,
        Drop,
    }

    pub struct Backend {
        join: Option<std::thread::JoinHandle<()>>,
        sender: std::sync::mpsc::Sender<StreamEvent>,
    }
    impl Backend {
        pub(super) fn start<G: Eq + Hash + Send + 'static>(
            mixer: Arc<Mutex<Mixer<G>>>,
        ) -> Result<Self, &'static str> {
            let (sender, receiver) = std::sync::mpsc::channel::<StreamEvent>();
            let join = {
                let sender = sender.clone();
                std::thread::spawn(move || {
                    log::trace!("starting thread");
                    StreamEventLoop {
                        mixer,
                        stream: None,
                    }
                    .run(sender, receiver)
                })
            };
            Ok(Self {
                join: Some(join),
                sender,
            })
        }
    }

    impl Drop for Backend {
        fn drop(&mut self) {
            self.sender.send(StreamEvent::Drop).unwrap();
            self.join.take().unwrap().join().unwrap();
        }
    }
}
#[cfg(target_arch = "wasm32")]
mod backend {
    use super::create_device;
    use crate::Mixer;
    use std::{
        hash::Hash,
        sync::{Arc, Mutex},
    };

    pub struct Backend {
        _stream: cpal::Stream,
    }
    impl Backend {
        pub(super) fn start<G: Eq + Hash + Send + 'static>(
            mixer: Arc<Mutex<Mixer<G>>>,
        ) -> Result<Self, &'static str> {
            // On Wasm backend, I cannot created a second thread to handle stream errors, but
            // errors in the wasm backend (AudioContext) is unexpected. In fact, cpal doesn't create
            // any StreamError in its wasm backend.
            let stream = create_device(&mixer, |err| log::error!("stream error: {err}"));
            let stream = match stream {
                Ok(x) => x,
                Err(x) => {
                    log::error!("creating audio device failed: {}", x);
                    return Err(x);
                }
            };
            Ok(Self { _stream: stream })
        }

        pub(super) fn resume(&self) {
            match self._stream.as_inner() {
                cpal::platform::StreamInner::WebAudio(x) => {
                    let _ = x.audio_context().resume();
                }
                #[allow(unreachable_patterns)]
                _ => {}
            }
        }
    }
}

/// The main struct of the crate.
///
/// This hold all existing `SoundSource`s and `cpal::platform::Stream`.
///
/// Each sound is associated with a group, which is purely used by
/// [`set_group_volume`](AudioEngine::set_group_volume), to allow mixing multiple sounds together.
pub struct AudioEngine<G: Eq + Hash + Send + 'static = ()> {
    mixer: Arc<Mutex<Mixer<G>>>,
    _backend: Backend,
}
impl<G: Default + Eq + Hash + Send> AudioEngine<G> {
    /// Create a new Sound in the default Group.
    ///
    /// Same as calling [`new_sound_with_group(G::default(), source)`](Self::new_sound_with_group).
    ///
    /// The added sound is started in stopped state, and [`play`](Sound::play) must be called to start playing
    /// it.
    ///
    /// Return a `Err` if the number of channels doesn't match the output number of channels. If
    /// the ouput number of channels is 1, or the number of channels of `source` is 1, `source`
    /// will be automatic wrapped in a [`ChannelConverter`]. If the `sample_rate` of `source`
    /// mismatch the output `sample_rate`, `source` will be wrapped in a [`SampleRateConverter`].
    pub fn new_sound<T: SoundSource + Send + 'static>(
        &self,
        source: T,
    ) -> Result<Sound<G>, &'static str> {
        self.new_sound_with_group(G::default(), source)
    }
}
impl AudioEngine {
    /// Tries to create a new AudioEngine.
    ///
    /// `cpal` will spawn a new thread where the sound samples will be sampled, mixed, and outputed
    /// to the output stream.
    pub fn new() -> Result<Self, &'static str> {
        AudioEngine::with_groups::<()>()
    }

    /// Tries to create a new AudioEngine, with the given type to represent sound groups.
    ///
    /// `cpal` will spawn a new thread where the sound samples will be sampled, mixed, and outputed
    /// to the output stream.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # fn main() -> Result<(), &'static str> {
    /// # let my_fx = audio_engine::SineWave::new(44100, 500.0);
    /// # let my_music = audio_engine::SineWave::new(44100, 440.0);
    /// use audio_engine::{AudioEngine, WavDecoder};
    ///
    /// #[derive(Eq, Hash, PartialEq)]
    /// enum Group {
    ///     Effect,
    ///     Music,
    /// }
    ///
    /// let audio_engine = AudioEngine::with_groups::<Group>()?;
    /// let mut fx = audio_engine.new_sound_with_group(Group::Effect, my_fx)?;
    /// let mut music = audio_engine.new_sound_with_group(Group::Music, my_music)?;
    ///
    /// fx.play();
    /// music.play();
    ///
    /// // decrease music volume, for example
    /// audio_engine.set_group_volume(Group::Music, 0.1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_groups<G: Eq + Hash + Send>() -> Result<AudioEngine<G>, &'static str> {
        let mixer = Arc::new(Mutex::new(Mixer::<G>::new(2, super::SampleRate(48000))));
        let backend = Backend::start(mixer.clone())?;

        Ok(AudioEngine::<G> {
            mixer,
            _backend: backend,
        })
    }
}
impl<G: Eq + Hash + Send> AudioEngine<G> {
    //// Call `resume()` on the underlying
    ///[`AudioContext`](https://developer.mozilla.org/pt-BR/docs/Web/API/AudioContext).
    ///
    /// On Chrome, if a `AudioContext` is created before a user interaction, the `AudioContext` will
    /// start in the "supended" state. To resume the `AudioContext`, `AudioContext.resume()` must be
    /// called.
    #[cfg(target_arch = "wasm32")]
    pub fn resume(&self) {
        self._backend.resume()
    }

    /// The sample rate that is currently being outputed to the device.
    pub fn sample_rate(&self) -> u32 {
        self.mixer.lock().unwrap().sample_rate()
    }

    /// The sample rate of the current output device.
    ///
    /// May change when the device changes.
    pub fn channels(&self) -> u16 {
        self.mixer.lock().unwrap().channels()
    }

    /// Create a new Sound with the given Group.
    ///
    /// Return a `Err` if the number of channels doesn't match the output number of channels. If
    /// the ouput number of channels is 1, or the number of channels of `source` is 1, `source`
    /// will be automatic wrapped in a [`ChannelConverter`].
    ///
    /// If the `sample_rate` of `source` mismatch the output `sample_rate`, `source` will be
    /// wrapped in a [`SampleRateConverter`].
    pub fn new_sound_with_group<T: SoundSource + Send + 'static>(
        &self,
        group: G,
        source: T,
    ) -> Result<Sound<G>, &'static str> {
        let mut mixer = self.mixer.lock().unwrap();

        let sound: Box<dyn SoundSource + Send> = if source.sample_rate() != mixer.sample_rate() {
            if source.channels() == mixer.channels() {
                Box::new(SampleRateConverter::new(source, mixer.sample_rate()))
            } else if mixer.channels() == 1 || source.channels() == 1 {
                Box::new(ChannelConverter::new(
                    SampleRateConverter::new(source, mixer.sample_rate()),
                    mixer.channels(),
                ))
            } else {
                return Err("Number of channels() do not match the output, and neither are 1");
            }
        } else if source.channels() == mixer.channels() {
            Box::new(source)
        } else if mixer.channels() == 1 || source.channels() == 1 {
            Box::new(ChannelConverter::new(source, mixer.channels()))
        } else {
            return Err("Number of channels() do not match the output, and is not 1");
        };

        let id = mixer.add_sound(group, sound);
        mixer.mark_to_remove(id, false);
        drop(mixer);

        Ok(Sound {
            mixer: self.mixer.clone(),
            id,
        })
    }

    /// Set the volume of the given group.
    ///
    /// The volume of all sounds associated with this group is multiplied by this volume.
    pub fn set_group_volume(&self, group: G, volume: f32) {
        self.mixer.lock().unwrap().set_group_volume(group, volume)
    }
}

fn create_device<G: Eq + Hash + Send + 'static>(
    mixer: &Arc<Mutex<Mixer<G>>>,
    error_callback: impl FnMut(StreamError) + Send + Clone + 'static,
) -> Result<cpal::Stream, &'static str> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("no output device available")?;
    let mut supported_configs_range = device
        .supported_output_configs()
        .map_err(|_| "error while querying formats")?
        .map(|x| {
            let sample_rate = SampleRate(48000);
            if x.min_sample_rate() <= sample_rate && sample_rate <= x.max_sample_rate() {
                return x.with_sample_rate(sample_rate);
            }

            let sample_rate = SampleRate(44100);
            if x.min_sample_rate() <= sample_rate && sample_rate <= x.max_sample_rate() {
                return x.with_sample_rate(sample_rate);
            }

            x.with_max_sample_rate()
        })
        .collect::<Vec<_>>();
    supported_configs_range.sort_unstable_by(|a, b| {
        let key = |x: &cpal::SupportedStreamConfig| {
            (
                x.sample_rate().0 == 48000,
                x.sample_rate().0 == 441000,
                x.channels() == 2,
                x.channels() == 1,
                x.sample_format() == cpal::SampleFormat::I16,
                x.sample_rate().0,
            )
        };
        key(a).cmp(&key(b))
    });
    if log::max_level() >= log::LevelFilter::Trace {
        for config in &supported_configs_range {
            log::trace!("config {:?}", config);
        }
    }
    let stream = loop {
        let config = if let Some(config) = supported_configs_range.pop() {
            config
        } else {
            return Err("no supported config");
        };
        let sample_format = config.sample_format();
        let config = config.config();
        mixer
            .lock()
            .unwrap()
            .set_config(config.channels, super::SampleRate(config.sample_rate.0));

        let stream = {
            use cpal::SampleFormat::*;
            match sample_format {
                I16 => stream::<i16, G, _>(mixer, error_callback.clone(), &device, &config),
                U16 => stream::<u16, G, _>(mixer, error_callback.clone(), &device, &config),
                F32 => stream::<f32, G, _>(mixer, error_callback.clone(), &device, &config),
            }
        };
        let stream = match stream {
            Ok(x) => {
                log::info!(
                    "created {:?} stream with config {:?}",
                    sample_format,
                    config
                );
                x
            }
            Err(e) => {
                log::error!("failed to create stream with config {:?}: {:?}", config, e);
                continue;
            }
        };
        stream.play().unwrap();
        break stream;
    };
    Ok(stream)
}

fn stream<T, G, E>(
    mixer: &Arc<Mutex<Mixer<G>>>,
    error_callback: E,
    device: &cpal::Device,
    config: &cpal::StreamConfig,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: cpal::Sample,
    G: Eq + Hash + Send + 'static,
    E: FnMut(StreamError) + Send + 'static,
{
    let mixer = mixer.clone();
    let mut input_buffer = Vec::new();
    device.build_output_stream(
        config,
        move |output_buffer: &mut [T], _| {
            input_buffer.clear();
            input_buffer.resize(output_buffer.len(), 0);
            mixer.lock().unwrap().write_samples(&mut input_buffer);
            // convert the samples from i16 to T, and write them in the output buffer.
            output_buffer
                .iter_mut()
                .zip(input_buffer.iter())
                .for_each(|(a, b)| *a = T::from(b));
        },
        error_callback,
    )
}
