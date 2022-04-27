# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
