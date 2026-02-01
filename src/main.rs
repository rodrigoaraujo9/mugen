use std::collections::HashMap;
use std::time::Duration;
use rodio::{OutputStreamBuilder, Sink};
use rodio::source::{SineWave, Source};
use device_query::{DeviceQuery, DeviceState, Keycode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
mod key;
use key::Key;



fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream_handle = OutputStreamBuilder::open_default_stream()
        .expect("open default audio stream");
    let mut active_sinks: HashMap<char, Sink> = HashMap::new();

    let device_state = DeviceState::new();
    let mut prev_keys: Vec<Keycode> = vec![];

    enable_raw_mode()?;

    loop {
        let keys = device_state.get_keys();

        if keys.contains(&Keycode::Escape) {
            break;
        }



        for keycode in &keys {
            if !prev_keys.contains(keycode) {
                if let Some(c) = Key::keycode_to_char(keycode) {
                    if let Some(key) = Key::from_keycode(c) {
                        let freq = key.frequency();
                        let sink = Sink::connect_new(&stream_handle.mixer());
                        let source = SineWave::new(freq)
                            .take_duration(Duration::from_secs(3600))
                            .amplify(0.20);
                        sink.append(source);
                        active_sinks.insert(c, sink);
                    }
                }
            }
        }

        for keycode in &prev_keys {
            if !keys.contains(keycode) {
                if let Some(c) = Key::keycode_to_char(keycode) {
                    if let Some(sink) = active_sinks.remove(&c) {
                        sink.stop();
                    }
                }
            }
        }

        prev_keys = keys;
    }

    disable_raw_mode()?;

    for (_, sink) in active_sinks {
        sink.stop();
    }

    Ok(())
}
