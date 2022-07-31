# Audio Engine

[![Crates.io](https://img.shields.io/crates/v/audio-engine.svg)](https://crates.io/crates/audio-engine) 
[![docs.rs](https://docs.rs/audio-engine/badge.svg)](https://docs.rs/audio-engine/)

A cross-platform rust crate for audio playback, build on top of cpal.

## Supported formats
- ogg
- wav

## Example

```rust
use audio_engine::{AudioEngine, WavDecoder};
let audio_engine = AudioEngine::new()?;
let mut sound = audio_engine.new_sound(WavDecoder::new(my_wav_sound)?)?;
sound.play();
```

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
