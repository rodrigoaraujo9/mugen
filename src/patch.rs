//! Constructs patch -> Builds voices from oscillator, ADSR, and modular effects

use rodio::Source;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::Effects::adsr::{Adsr, AdsrHandle, adsr};
use crate::Oscilators::basic::{OscHandle, Wave, osc_source};

pub type Sample = f32;
pub type PatchSource = Box<dyn Source<Item = Sample> + Send>;
pub type Gate = Arc<AtomicBool>;

pub trait Effect: Send + Sync {
    fn name(&self) -> &'static str;
    fn apply(&self, input: PatchSource) -> PatchSource;
}

pub type SharedEffect = Arc<dyn Effect>;

#[macro_export]
macro_rules! impl_source_passthrough {
    ($ty:ty, $input:ident) => {
        impl rodio::Source for $ty {
            fn current_span_len(&self) -> Option<usize> {
                self.$input.current_span_len()
            }

            fn channels(&self) -> u16 {
                self.$input.channels()
            }

            fn sample_rate(&self) -> u32 {
                self.$input.sample_rate()
            }

            fn total_duration(&self) -> Option<std::time::Duration> {
                self.$input.total_duration()
            }
        }
    };
}

#[derive(Clone)]
pub struct Patch {
    osc: OscHandle,
    adsr: AdsrHandle,
    effects: Vec<SharedEffect>,
}

impl Patch {
    pub fn new(osc: OscHandle, adsr: AdsrHandle, effects: Vec<SharedEffect>) -> Self {
        Self { osc, adsr, effects }
    }

    #[inline]
    pub fn build_voice(&self, frequency: f32, gate: Gate) -> PatchSource {
        let mut source: PatchSource = Box::new(osc_source(frequency, self.osc.clone()));

        for effect in &self.effects {
            source = effect.apply(source);
        }

        adsr(source, self.adsr.clone(), gate)
    }

    #[inline]
    pub fn wave(&self) -> Wave {
        self.osc.get().wave
    }

    #[inline]
    pub fn set_wave(&self, wave: Wave) {
        self.osc.update(|osc| osc.wave = wave);
    }

    #[inline]
    pub fn toggle_wave(&self) {
        self.osc.update(|osc| osc.wave = osc.wave.toggle());
    }

    #[inline]
    pub fn adsr(&self) -> Adsr {
        self.adsr.get()
    }

    #[inline]
    pub fn set_adsr(&self, adsr_value: Adsr) {
        self.adsr.set(adsr_value);
    }

    #[inline]
    pub fn name(&self) -> String {
        let mut out = self.wave().name().to_string();

        if !self.effects.is_empty() {
            out.push_str(" | ");
            out.push_str(
                &self
                    .effects
                    .iter()
                    .map(|effect| effect.name())
                    .collect::<Vec<_>>()
                    .join(" -> "),
            );
        }

        out
    }
}
