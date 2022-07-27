# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

# Unreleased

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
