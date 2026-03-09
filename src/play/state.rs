use crate::nodes::adsr::Adsr;
use crate::nodes::lfo_amp::LfoAmp;
use crate::nodes::lowpass::LowPass;
use crate::patch::{Gate, Generator};
use device_query::Keycode;
use rodio::Sink;
use rodio::stream::{OutputStream, OutputStreamBuilder};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::Ordering;

pub type ActiveNote = (Sink, Gate);

pub struct PlayState {
    pub stream: OutputStream,
    pub active_sinks: HashMap<Keycode, Vec<ActiveNote>>,
}

impl PlayState {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let stream = OutputStreamBuilder::open_default_stream()?;
        Ok(Self {
            stream,
            active_sinks: HashMap::new(),
        })
    }

    pub fn stop_note(&mut self, keycode: Keycode) {
        if let Some(voices) = self.active_sinks.get_mut(&keycode) {
            for (_sink, gate) in voices.iter_mut() {
                gate.store(false, Ordering::Relaxed);
            }
        }
    }

    pub fn kill_all(&mut self) {
        for (_k, mut voices) in self.active_sinks.drain() {
            for (sink, gate) in voices.drain(..) {
                gate.store(false, Ordering::Relaxed);
                sink.stop();
            }
        }
    }

    pub fn cleanup_finished(&mut self) {
        self.active_sinks.retain(|_, voices| {
            voices.retain(|(sink, _)| !sink.empty());
            !voices.is_empty()
        });
    }

    pub fn set_all_volume(&mut self, v: f32) {
        for (_k, voices) in self.active_sinks.iter_mut() {
            for (sink, _gate) in voices.iter_mut() {
                sink.set_volume(v);
            }
        }
    }

    pub fn set_all_muted(&mut self, muted: bool) {
        for (_k, voices) in self.active_sinks.iter_mut() {
            for (sink, _gate) in voices.iter_mut() {
                if muted {
                    sink.pause();
                } else {
                    sink.play();
                }
            }
        }
    }
}

pub struct RuntimeState {
    pub volume: f32,
    pub muted: bool,
    pub adsr: Adsr,
    pub lfo: LfoAmp,
    pub lowpass: LowPass,
    pub available_generators: Vec<Box<dyn Generator>>,
    pub current_gen_idx: usize,
    pub held_keys: HashSet<Keycode>,
    pub octave_offset: i32,
}

impl RuntimeState {
    pub fn current_generator(&self) -> Option<&dyn Generator> {
        self.available_generators
            .get(self.current_gen_idx)
            .map(|g| g.as_ref())
    }
}
