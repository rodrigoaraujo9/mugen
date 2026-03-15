use crate::audio::AudioSnapshot;
use crate::generators::basic::{BasicGenerator, basic_generator};
use crate::nodes::adsr::Adsr;
use crate::nodes::lfo_amp::LfoAmp;
use crate::nodes::lowpass::LowPass;
use crate::patch::{Gate, Patch, SharedGenerator, SharedNode};
use device_query::Keycode;
use rodio::Sink;
use rodio::stream::{OutputStream, OutputStreamBuilder};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};

pub type ActiveNote = (Sink, Gate);

pub struct PlayState {
    pub stream: OutputStream,
    pub active_sinks: HashMap<Keycode, Vec<ActiveNote>>,
}

impl PlayState {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            stream: OutputStreamBuilder::open_default_stream()?,
            active_sinks: HashMap::new(),
        })
    }

    pub fn stop_note(&mut self, keycode: Keycode) {
        if let Some(voices) = self.active_sinks.get_mut(&keycode) {
            for (_, gate) in voices.iter_mut() {
                gate.store(false, Ordering::Relaxed);
            }
        }
    }

    pub fn kill_all(&mut self) {
        for (_, mut voices) in self.active_sinks.drain() {
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

    pub fn set_all_volume(&mut self, volume: f32) {
        for voices in self.active_sinks.values_mut() {
            for (sink, _) in voices {
                sink.set_volume(volume);
            }
        }
    }

    pub fn set_all_muted(&mut self, muted: bool) {
        for voices in self.active_sinks.values_mut() {
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

    pub adsr: Arc<RwLock<Adsr>>,
    pub generator: Arc<BasicGenerator>,
    pub lfo: LfoAmp,
    pub lowpass: LowPass,
    pub patch: Arc<Patch>,
}

impl RuntimeState {
    pub fn new(snapshot: AudioSnapshot) -> Self {
        let basic_generator = basic_generator(snapshot.generator_kind);
        let generator: SharedGenerator = basic_generator.clone();

        let lfo = LfoAmp::from_params(snapshot.lfo);
        let lowpass = LowPass::from_params(snapshot.lowpass);

        let patch = Arc::new(Patch::new(generator));
        patch.set_nodes(vec![
            Arc::new(lfo.clone()) as SharedNode,
            Arc::new(lowpass.clone()) as SharedNode,
        ]);

        Self {
            volume: snapshot.volume,
            muted: snapshot.muted,
            octave_offset: 0,
            held_keys: HashSet::new(),
            adsr: Arc::new(RwLock::new(snapshot.adsr)),
            generator: basic_generator,
            lfo,
            lowpass,
            patch,
        }
    }

    #[inline]
    pub fn adsr(&self) -> Adsr {
        *self.adsr.read().unwrap()
    }

    #[inline]
    pub fn generator_kind(&self) -> crate::generators::basic::BasicKind {
        self.generator.params().kind
    }

    #[inline]
    pub fn patch_name(&self) -> String {
        self.generator_kind().name().to_string()
    }

    #[inline]
    pub fn snapshot(&self) -> AudioSnapshot {
        AudioSnapshot {
            volume: self.volume,
            muted: self.muted,
            generator_kind: self.generator_kind(),
            patch_name: self.patch_name(),
            adsr: self.adsr(),
            lfo: self.lfo.params(),
            lowpass: self.lowpass.params(),
        }
    }
}
