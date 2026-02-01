use device_query::{DeviceQuery, DeviceState, Keycode};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use rodio::stream::{OutputStreamBuilder, OutputStream};
use rodio::Sink;
use tokio::signal::ctrl_c;
use tokio::task;
use tokio::sync::Notify;
use std::sync::Arc;
use crate::config::TICK;
use crate::key::Key;
use crate::state;
use crate::audio_capture::AudioCapture;

pub struct Play {
    _stream: OutputStream,
    active_sinks: HashMap<Keycode, Sink>,
    volume_notify: Arc<Notify>,
    pause_notify: Arc<Notify>,
    pub audio_capture: Arc<AudioCapture>,
}

impl Play {
    pub fn new(channels: usize, buffer_size: usize, sample_rate: u32) -> Result<Self, Box<dyn std::error::Error>> {
        let stream = OutputStreamBuilder::open_default_stream()?;
        Ok(Self {
            _stream: stream,
            active_sinks: HashMap::new(),
            volume_notify: Arc::new(Notify::new()),
            pause_notify: Arc::new(Notify::new()),
            audio_capture: Arc::new(AudioCapture::new(channels, buffer_size, sample_rate)),
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

            let channels = audio_source.channels() as usize;
            let tapped_source = self.audio_capture.create_tap_source(audio_source, channels);

            let volume = state::get_volume().await;
            sink.set_volume(volume);
            if state::is_muted().await {
                sink.pause();
            }
            sink.append(tapped_source);
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

    pub async fn sync_muted_state(&mut self) {
        if state::is_muted().await {
            for sink in self.active_sinks.values_mut() {
                sink.pause();
            }
        } else {
            for sink in self.active_sinks.values_mut() {
                sink.play();
            }
        }
    }

    pub fn get_volume_notify(&self) -> Arc<Notify> {
        Arc::clone(&self.volume_notify)
    }

    pub fn get_muted_notify(&self) -> Arc<Notify> {
        Arc::clone(&self.pause_notify)
    }

    pub fn get_audio_capture(&self) -> Arc<AudioCapture> {
        Arc::clone(&self.audio_capture)
    }
}

impl Drop for Play {
    fn drop(&mut self) {
        self.stop_all();
    }
}

pub async fn run_audio() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let shutdown = Arc::new(Notify::new());

    let poll_handle = task::spawn_blocking(move || {
        let device_state = DeviceState::new();
        let mut prev: HashSet<Keycode> = HashSet::new();

        loop {
            std::thread::sleep(Duration::from_millis(TICK));
            let now: HashSet<Keycode> = device_state.get_keys().into_iter().collect();

            if now.contains(&Keycode::Escape) ||
               (now.contains(&Keycode::C) && now.contains(&Keycode::LControl)) {
                let _ = tx.send(None);
                break;
            }

            if now != prev {
                if tx.send(Some((now.clone(), prev.clone()))).is_err() {
                    break;
                }
                prev = now;
            }
        }
    });

    let mut audio = Play::new(2, 2048, 48000)?;
    let volume_notify = audio.get_volume_notify();
    let pause_notify = audio.get_muted_notify();
    let audio_capture = audio.get_audio_capture();

    state::set_volume_notify(volume_notify).await;
    state::set_mute_notify(pause_notify).await;
    state::set_audio_capture(audio_capture).await;

    let ctrl_c = ctrl_c();
    tokio::pin!(ctrl_c);

    loop {
        tokio::select! {
            _ = &mut ctrl_c => {
                shutdown.notify_one();
                break;
            }
            msg = rx.recv() => {
                match msg {
                    Some(Some((now, prev))) => {
                        for k in now.difference(&prev) {
                            audio.play_note(*k).await;
                        }
                        for k in prev.difference(&now) {
                            audio.stop_note(*k);
                        }
                    }
                    Some(None) | None => break,
                }
            }
            _ = audio.volume_notify.notified() => {
                audio.sync_volume().await;
            }
            _ = audio.pause_notify.notified() => {
                audio.sync_muted_state().await;
            }
        }
    }

    audio.stop_all();
    let _ = poll_handle.await;

    Ok(())
}
