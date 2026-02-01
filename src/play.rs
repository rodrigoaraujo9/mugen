use device_query::{DeviceQuery, DeviceState, Keycode};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use rodio::stream::OutputStreamBuilder;
use rodio::Sink;
use tokio::time::interval;
use tokio::signal::ctrl_c;
use crate::config::TICK;
use crate::key::Key;
use crate::state;

pub struct Play {
    _stream: rodio::OutputStream,
    active_sinks: HashMap<Keycode, Sink>,
}

impl Play {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let stream = OutputStreamBuilder::open_default_stream()?;
        Ok(Self {
            _stream: stream,
            active_sinks: HashMap::new(),
        })
    }

    pub async fn play_note(&mut self, keycode: Keycode) {
        if self.active_sinks.contains_key(&keycode) {
            return;
        }

        if let Some(key) = Key::from_keycode(keycode) {
            let freq = key.frequency();
            let sink = Sink::connect_new(&self._stream.mixer());

            let source = state::get_source().await;
            let src = source.read().await;
            let audio_source = src.create_source(freq);

            let volume = state::get_volume().await;
            sink.set_volume(volume);

            if state::is_paused().await {
                sink.pause();
            }

            sink.append(audio_source);
            self.active_sinks.insert(keycode, sink);
        }
    }

    pub fn stop_note(&mut self, keycode: Keycode) {
        if let Some(sink) = self.active_sinks.remove(&keycode) {
            sink.stop();
        }
    }

    pub fn stop_all(&mut self) {
        for (_, sink) in self.active_sinks.drain() {
            sink.stop();
        }
    }

    pub async fn sync_volume(&mut self) {
        let volume = state::get_volume().await;
        for sink in self.active_sinks.values_mut() {
            sink.set_volume(volume);
        }
    }

    pub async fn sync_pause_state(&mut self) {
        if state::is_paused().await {
            for sink in self.active_sinks.values_mut() {
                sink.pause();
            }
        } else {
            for sink in self.active_sinks.values_mut() {
                sink.play();
            }
        }
    }
}

impl Drop for Play {
    fn drop(&mut self) {
        self.stop_all();
    }
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut audio = Play::new()?;
    let device_state = DeviceState::new();
    let mut prev: HashSet<Keycode> = HashSet::new();
    let mut tick = interval(Duration::from_millis(TICK));
    let ctrl_c = ctrl_c();
    tokio::pin!(ctrl_c);

    loop {
        tokio::select! {
            _ = &mut ctrl_c => break,
            _ = tick.tick() => {
                let now: HashSet<Keycode> = device_state.get_keys().into_iter().collect();

                if now.contains(&Keycode::Escape) ||
                   (now.contains(&Keycode::C) && now.contains(&Keycode::LControl)) {
                    break;
                }

                if now == prev {
                    continue;
                }

                for k in now.difference(&prev) {
                    audio.play_note(*k).await;
                }

                for k in prev.difference(&now) {
                    audio.stop_note(*k);
                }

                prev = now;
            }
        }
    }

    Ok(())
}
