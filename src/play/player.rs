//! Playback engine responsible for active sinks, note lifecycle, and stream control

use crate::patch::Gate;
use device_query::Keycode;
use rodio::Sink;
use rodio::stream::{OutputStream, OutputStreamBuilder};
use std::collections::HashMap;
use std::sync::atomic::Ordering;

pub type ActiveNote = (Sink, Gate);

pub struct Player {
    pub stream: OutputStream,
    pub voices: HashMap<Keycode, Vec<ActiveNote>>,
}

impl Player {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            stream: OutputStreamBuilder::open_default_stream()?,
            voices: HashMap::new(),
        })
    }

    pub fn stop_note(&mut self, keycode: Keycode) {
        if let Some(voices) = self.voices.get_mut(&keycode) {
            for (_, gate) in voices {
                gate.store(false, Ordering::Relaxed);
            }
        }
    }

    pub fn kill_all(&mut self) {
        for (_, mut voices) in self.voices.drain() {
            for (sink, gate) in voices.drain(..) {
                gate.store(false, Ordering::Relaxed);
                sink.stop();
            }
        }
    }

    pub fn cleanup_finished(&mut self) {
        self.voices.retain(|_, voices| {
            voices.retain(|(sink, _)| !sink.empty());
            !voices.is_empty()
        });
    }

    pub fn set_all_volume(&mut self, volume: f32) {
        for voices in self.voices.values_mut() {
            for (sink, _) in voices {
                sink.set_volume(volume);
            }
        }
    }

    pub fn set_all_muted(&mut self, muted: bool) {
        for voices in self.voices.values_mut() {
            for (sink, _) in voices {
                if muted {
                    sink.pause();
                } else {
                    sink.play();
                }
            }
        }
    }
}
