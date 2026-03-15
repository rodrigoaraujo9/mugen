use crate::audio::AudioSnapshot;
use crate::generators::basic::{BasicGenerator, BasicKind, basic_generator};
use crate::nodes::adsr::Adsr;
use crate::nodes::lfo_amp::LfoAmp;
use crate::nodes::lowpass::LowPass;
use crate::patch::{Gate, Patch, SharedGenerator, SharedNode};
use device_query::Keycode;
use rodio::Sink;
use rodio::stream::{OutputStream, OutputStreamBuilder};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::RwLock;
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
        for voices in self.active_sinks.values_mut() {
            for (sink, _) in voices.iter_mut() {
                sink.set_volume(v);
            }
        }
    }

    pub fn set_all_muted(&mut self, muted: bool) {
        for voices in self.active_sinks.values_mut() {
            for (sink, _) in voices.iter_mut() {
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
    pub adsr: Arc<RwLock<Adsr>>,
    pub basic_generator: Arc<BasicGenerator>,
    pub lfo: LfoAmp,
    pub lowpass: LowPass,
    pub patch: Arc<Patch>,
    pub held_keys: HashSet<Keycode>,
    pub octave_offset: i32,
}

impl RuntimeState {
    pub fn new(initial: AudioSnapshot) -> Self {
        let basic_generator = basic_generator(BasicKind::Sine);
        let generator: SharedGenerator = basic_generator.clone();

        let lfo = LfoAmp::from_params(initial.lfo);
        let lowpass = LowPass::from_params(initial.lowpass);

        let patch = Arc::new(Patch::new(generator));

        let lfo_node: SharedNode = Arc::new(lfo.clone());
        let lowpass_node: SharedNode = Arc::new(lowpass.clone());
        patch.set_nodes(vec![lfo_node, lowpass_node]);

        Self {
            volume: initial.volume,
            muted: initial.muted,
            adsr: Arc::new(RwLock::new(initial.adsr)),
            basic_generator,
            lfo,
            lowpass,
            patch,
            held_keys: HashSet::new(),
            octave_offset: 0,
        }
    }

    pub fn patch_name(&self) -> String {
        self.basic_generator.params().kind.name().to_string()
    }

    pub fn adsr_value(&self) -> Adsr {
        *self.adsr.read().unwrap()
    }
}
