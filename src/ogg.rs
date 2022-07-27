use lewton::inside_ogg::OggStreamReader;
use std::{
    io::{Read, Seek, SeekFrom},
    vec::IntoIter,
};

use crate::SoundSource;

/// A SourceSource, from ogg encoded sound data.
pub struct OggDecoder<T: Seek + Read + Send + 'static> {
    reader: Option<OggStreamReader<T>>,
    buffer: IntoIter<i16>,
    done: bool,
}
impl<T: Seek + Read + Send + 'static> OggDecoder<T> {
    /// Create a new OggDecoder from the given .ogg data.
    pub fn new(data: T) -> Result<Self, lewton::VorbisError> {
        let mut reader = OggStreamReader::new(data)?;
        // The first packed is always empty
        let _ = reader.read_dec_packet_itl()?;
        Ok(Self {
            buffer: reader
                .read_dec_packet_itl()?
                .unwrap_or_default()
                .into_iter(),
            reader: Some(reader),
            done: false,
        })
    }

    fn reader(&self) -> &OggStreamReader<T> {
        self.reader.as_ref().unwrap()
    }

    fn reader_mut(&mut self) -> &mut OggStreamReader<T> {
        self.reader.as_mut().unwrap()
    }
}
impl<T: Seek + Read + Send + 'static> SoundSource for OggDecoder<T> {
    fn channels(&self) -> u16 {
        self.reader().ident_hdr.audio_channels as u16
    }

    fn sample_rate(&self) -> u32 {
        self.reader().ident_hdr.audio_sample_rate
    }

    fn reset(&mut self) {
        let reader = self.reader.take();
        let mut source = reader.unwrap().into_inner().into_inner();
        source.seek(SeekFrom::Start(0)).unwrap();
        let reader = OggStreamReader::new(source).unwrap();
        self.reader = Some(reader);
        self.done = false;
        // The first packed is always empty
        let _ = self.reader_mut().read_dec_packet_itl().unwrap();
        self.buffer = self
            .reader_mut()
            .read_dec_packet_itl()
            .unwrap()
            .unwrap_or_default()
            .into_iter();
    }
    fn write_samples(&mut self, buffer: &mut [i16]) -> usize {
        let mut i = 0;

        'main: while i < buffer.len() {
            if let Some(next) = self.buffer.next() {
                buffer[i] = next;
                i += 1;
            } else {
                while let Some(pck) = self.reader_mut().read_dec_packet_itl().unwrap() {
                    if !pck.is_empty() {
                        self.buffer = pck.into_iter();
                        continue 'main;
                    }
                }
                return i;
            }
        }

        buffer.len()
    }
}
