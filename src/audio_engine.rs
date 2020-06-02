use std::thread;
use std::sync::mpsc;
use std::sync::{ Arc, Mutex };
use cpal::traits::{ DeviceTrait, HostTrait, EventLoopTrait };
use cpal::{ StreamData, UnknownTypeOutputBuffer };

use crate::converter::{ChannelConverter, SampleRateConverter};
use super::{ Mixer, Sound, SoundSource };

pub struct AudioEngine {
    mixer: Arc<Mutex<Mixer>>,
    channels: u16,
    sample_rate: u32,
}
impl AudioEngine {
    pub fn new() -> Result<Self, &'static str> {

        let (tx, rx) = mpsc::channel();

        thread::Builder::new()
            .name("AudioEngine: cpal event loop".to_owned())
            .spawn(move || {
                let result = || {
                    let host = cpal::default_host();
                    let event_loop = host.event_loop();
                    let device = host.default_output_device()
                        .ok_or("no output device available")?;
                    let mut supported_formats_range = device.supported_output_formats()
                        .map_err(|_| "error while querying formats")?;
                    let format = supported_formats_range.next()
                        .ok_or("no supported format?!")?
                        .with_max_sample_rate();

                    let channels = format.channels;
                    let sample_rate = format.sample_rate.0;
                    
                    Ok((
                        Arc::new(Mutex::new(
                            Mixer::new(channels, sample_rate)
                        )),
                        event_loop,
                        device,
                        format
                    ))
                };
                let result = result();

                let (mixer, event_loop, device, format) = match result {
                    Ok(ok) => {
                        tx.send(Ok(ok.0.clone())).unwrap();
                        ok
                    },
                    Err(err) => {
                        tx.send(Err(err)).unwrap();
                        return;
                    }
                };
                drop(tx);

                let stream_id = event_loop.build_output_stream(&device, &format).unwrap();
                event_loop.play_stream(stream_id).expect("failed to play_stream");

                event_loop.run(|id, result| {
                    let stream_data = match result {
                        Ok(data) => data,
                        Err(err) => {
                            eprintln!("an error occurred on stream {:?}: {}", id, err);
                            return;
                        }
                    };
                    let mut mixer = mixer.lock().unwrap();
                    match stream_data {
                        StreamData::Input {..} => {}
                        StreamData::Output { buffer } => {
                            match buffer {
                                UnknownTypeOutputBuffer::U16(mut buffer) => {
                                    let mut buf = vec![0; buffer.len()];
                                    mixer.write_samples(&mut buf);
                                    for i in 0..buffer.len() {
                                        buffer[i] = (buf[i] as u16).wrapping_add(i16::max_value() as u16);
                                    }
                                }
                                UnknownTypeOutputBuffer::I16(mut buffer) => {
                                    mixer.write_samples(&mut buffer);
                                }
                                UnknownTypeOutputBuffer::F32(mut buffer) => {
                                    let mut buf = vec![0; buffer.len()];
                                    mixer.write_samples(&mut buf);
                                    for i in 0..buffer.len() {
                                        buffer[i] = buf[i] as f32 / i16::max_value() as f32;
                                    }
                                }
                            }
                        }
                    }
                });
        }).map_err(|_| "Thread Spawn failed.")?;

        match rx.recv().unwrap() {
            Ok(mixer) => {
                let m = mixer.lock().unwrap();
                let channels = m.channels;
                let sample_rate = m.sample_rate;
                drop(m);
                Ok(Self {
                    mixer,
                    channels,
                    sample_rate,
                })
            },
            Err(err) => Err(err),
        }
    }
    
    pub fn new_sound<T: SoundSource + Send + 'static>(&self, source: T) -> Result<Sound, &'static str> {
        let sound: Box<dyn SoundSource + Send> = if source.sample_rate() != self.sample_rate {
            if source.channels() == self.channels {
                Box::new(SampleRateConverter::new(source, self.sample_rate))
            } else if source.channels() == 1 {
                Box::new( ChannelConverter::new(
                    SampleRateConverter::new(source, self.sample_rate),
                    self.channels
                ))
            } else {
                return Err("Number of channels do not match the output, and is not 1");
            }
        } else if source.channels() == self.channels {
            Box::new(source)
        } else if source.channels() == 1 {
            Box::new( ChannelConverter::new(source, self.channels))
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