# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# Unreleased

### Changed

- Make `AudioEngine` be `Sync`.
- **breaking**: `AudioEngine::resume` now takes a exclusive `&mut self` reference.

# [0.4.5] - 2022-12-19

### Fixed

- Make `ChannelConverter` don't panic when the number of channels of input and
  output were both not 1.
- Fixed #7: `stop()` was not working when all sounds were playing.

# [0.4.4] - 2022-10-11

*Should have included the fix for #7, but because of a confusion with git, the
fix as not included in the publication of this version. This version is
identical to the previous.*

# [0.4.3] - 2022-10-11

### Changed

- Put `OggDecoder` and `WavDecoder`, and respective dependencies, behind the
  features `ogg` and `wav`. Both are enabled by default.

# [0.4.2] - 2022-09-07

### Changed

- Updated `cpal` to 0.14.0

### Added

- Wasm: add `AudioEngine::resume`, for resuming the underlying AudioContext,
  which can be in "suspended" state on Chrome.

# [0.4.1] - 2022-08-18

### Fixed

- Android: work around crash when dropping `AudioEngine` due to unsoundness in oboe-rs.

# [0.4.0] - 2022-07-31

### Added

- Make `Mixer` be public.
- implement `SoundSource` for `Arc<Mutex<T>>`.
- Add the `SineWave` `SoundSource`.
- Add sound groups to AudioEngine:

``` rust
use audio_engine::{AudioEngine, WavDecoder};

#[derive(Eq, Hash, PartialEq)]
enum Group {
    Effect,
    Music,
}

let audio_engine = AudioEngine::with_groups::<Group>()?;
let mut fx = audio_engine.new_sound_with_group(Group::Effect, my_fx)?;
let mut music = audio_engine.new_sound_with_group(Group::Music, my_music)?;

fx.play();
music.play();

// decrease music volume, for example
audio_engine.set_group_volume(Group::Music, 0.1);
```

### Fixed

- Fix `set_volume` don't working for more than one added sounds (#5).

## [0.3.0] - 2022-07-03

### Changed

- **breaking**: Return a `Result` instead of panicking if creating a
  `WavDecoder` or `OggDecoder` fail.

### Fixed

- Handle i8, i16, i24, i32 and f32 sample formats in `WavDecoder`.

## [0.2.3] - 2022-06-29

### Fixed

- Fix compiler error in WebAssembly.

## [0.2.2] - 2022-06-09

### Added

- Add Android support.
- Handle device disconnection.

## [0.2.1] - 2022-04-27

### Fixed

- Enable `cpal`'s feature `wasm-bindgen` by default.

## [0.2.0] - 2022-04-27

### Changed

- **breaking**: Remove custom WebAssembly backend, replacing it with cpal's
  WebAudio backend (that don't require calling update regularly).

### Fixed

- Fix a panic in `SampleRateConverter` and add unit tests.

## [0.1.0] - 2022-03-31
