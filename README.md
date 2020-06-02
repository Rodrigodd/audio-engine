#TODO

- Handle the channel conversion more wisely
- Refactor the SampleRate conversion
- Create a ```type MixerRef = Arc<Mutex<Mixer>> | Rc<Cell<Mixer>>'``` for native and
web diferences?