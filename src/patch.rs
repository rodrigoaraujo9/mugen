use rodio::Source;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::generators::basic::{OscParamsHandle, osc};
use crate::nodes::adsr::AdsrHandle;
use crate::nodes::gain::gain;
use crate::nodes::lfo_amp::{LfoAmpHandle, lfo_amp};
use crate::nodes::lowpass::{LowPassHandle, lowpass};

pub type Sample = f32;
pub type SynthSource = Box<dyn Source<Item = Sample> + Send>;
pub type Gate = Arc<AtomicBool>;

#[derive(Clone)]
pub struct Patch {
    pub osc: OscParamsHandle,
    pub adsr: AdsrHandle,
    pub lfo: LfoAmpHandle,
    pub lowpass: LowPassHandle,
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
    pub fn voice(&self, frequency: f32, gate: Gate) -> SynthSource {
        osc(frequency, self.osc.clone())
            .lfo_amp(self.lfo.clone())
            .lowpass(self.lowpass.clone())
            .adsr(self.adsr.clone(), gate)
    }

    #[inline]
    pub fn name(&self) -> String {
        self.osc.get().kind.name().to_string()
    }
}

pub trait SourceChain: Sized
where
    Self: Source<Item = f32> + Send + 'static,
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

impl<T> SourceChain for T where T: Source<Item = f32> + Send + 'static {}
