use crate::audio::AudioSnapshot;
use crate::generators::basic::{Wave, osc_params};
use crate::nodes::adsr::{Adsr, adsr_handle};
use crate::nodes::lfo_amp::{LfoAmpParams, lfo_amp_handle};
use crate::nodes::lowpass::{LowPassParams, lowpass_handle};
use crate::patch::{Gate, Patch};
use device_query::Keycode;
use rodio::Sink;
use rodio::stream::{OutputStream, OutputStreamBuilder};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::Ordering;

pub type ActiveNote = (Sink, Gate);

pub struct PlayState {
    pub stream: OutputStream,
    pub voices: HashMap<Keycode, Vec<ActiveNote>>,
}

impl PlayState {
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

pub struct RuntimeState {
    pub volume: f32,
    pub muted: bool,
    pub octave_offset: i32,
    pub held_keys: HashSet<Keycode>,
    pub patch: Patch,
}

impl RuntimeState {
    pub fn new(snapshot: AudioSnapshot) -> Self {
        let osc = osc_params(snapshot.wave_kind);
        let adsr = adsr_handle(snapshot.adsr);
        let lfo = lfo_amp_handle(snapshot.lfo);
        let lowpass = lowpass_handle(snapshot.lowpass);

        Self {
            volume: snapshot.volume,
            muted: snapshot.muted,
            octave_offset: 0,
            held_keys: HashSet::new(),
            patch: Patch::new(osc, adsr, lfo, lowpass),
        }
    }

    #[inline]
    pub fn osc_kind(&self) -> Wave {
        self.patch.osc.get().kind
    }

    #[inline]
    pub fn adsr(&self) -> Adsr {
        self.patch.adsr.get()
    }

    #[inline]
    pub fn lfo(&self) -> LfoAmpParams {
        self.patch.lfo.get()
    }

    #[inline]
    pub fn lowpass(&self) -> LowPassParams {
        self.patch.lowpass.get()
    }

    #[inline]
    pub fn snapshot(&self) -> AudioSnapshot {
        AudioSnapshot {
            volume: self.volume,
            muted: self.muted,
            wave_kind: self.osc_kind(),
            patch_name: self.patch.name(),
            adsr: self.adsr(),
            lfo: self.lfo(),
            lowpass: self.lowpass(),
        }
    }
}
