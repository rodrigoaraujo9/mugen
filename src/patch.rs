use rodio::Source;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::generators::basic::{OscParamsHandle, Wave, osc};
use crate::nodes::adsr::{Adsr, AdsrHandle};
use crate::nodes::gain::gain;
use crate::nodes::lfo_amp::{LfoAmpHandle, LfoAmpParams, lfo_amp};
use crate::nodes::lowpass::{LowPassHandle, LowPassParams, lowpass};

pub type Sample = f32;
pub type SynthSource = Box<dyn Source<Item = Sample> + Send>;
pub type Gate = Arc<AtomicBool>;

#[derive(Clone)]
pub struct Patch {
    osc: OscParamsHandle,
    adsr: AdsrHandle,
    lfo: LfoAmpHandle,
    lowpass: LowPassHandle,
}

impl Patch {
    pub fn new(
        osc: OscParamsHandle,
        adsr: AdsrHandle,
        lfo: LfoAmpHandle,
        lowpass: LowPassHandle,
    ) -> Self {
        Self {
            osc,
            adsr,
            lfo,
            lowpass,
        }
    }

    #[inline]
    pub fn build_voice(&self, frequency: f32, gate: Gate) -> SynthSource {
        osc(frequency, self.osc.clone())
            .lfo_amp(self.lfo.clone())
            .lowpass(self.lowpass.clone())
            .adsr(self.adsr.clone(), gate)
    }

    #[inline]
    pub fn name(&self) -> String {
        self.wave().name().to_string()
    }

    #[inline]
    pub fn wave(&self) -> Wave {
        self.osc.get().kind
    }

    #[inline]
    pub fn adsr(&self) -> Adsr {
        self.adsr.get()
    }

    #[inline]
    pub fn lfo(&self) -> LfoAmpParams {
        self.lfo.get()
    }

    #[inline]
    pub fn lowpass(&self) -> LowPassParams {
        self.lowpass.get()
    }

    #[inline]
    pub fn set_wave(&self, wave: Wave) {
        self.osc.update(|params| params.kind = wave);
    }

    #[inline]
    pub fn set_adsr(&self, adsr: Adsr) {
        self.adsr.set(adsr);
    }

    #[inline]
    pub fn set_lfo(&self, params: LfoAmpParams) {
        self.lfo.set(params);
    }

    #[inline]
    pub fn set_lowpass(&self, params: LowPassParams) {
        self.lowpass.set(params);
    }

    #[inline]
    pub fn toggle_wave(&self) {
        self.osc.update(|params| params.kind = params.kind.toggle());
    }
}

pub trait SourceChain: Sized
where
    Self: Source<Item = Sample> + Send + 'static,
{
    #[inline]
    fn boxed(self) -> SynthSource {
        Box::new(self)
    }

    #[inline]
    fn lfo_amp(self, params: LfoAmpHandle) -> SynthSource {
        lfo_amp(Box::new(self), params)
    }

    #[inline]
    fn lowpass(self, params: LowPassHandle) -> SynthSource {
        lowpass(Box::new(self), params)
    }

    #[inline]
    fn adsr(self, adsr: AdsrHandle, gate: Gate) -> SynthSource {
        crate::nodes::adsr::adsr(Box::new(self), adsr, gate)
    }

    #[inline]
    fn gain(self, params: crate::nodes::gain::GainHandle) -> SynthSource {
        gain(Box::new(self), params)
    }
}

impl<T> SourceChain for T where T: Source<Item = Sample> + Send + 'static {}
