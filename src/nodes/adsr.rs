use crate::patch::{Gate, SynthSource};
use crate::shared::Shared;
use rodio::Source;
use std::sync::atomic::Ordering;
use std::time::Duration;

#[derive(Clone, Copy, Debug)]
pub struct Adsr {
    pub attack_s: f32,
    pub decay_s: f32,
    pub sustain: f32,
    pub release_s: f32,
}

impl Adsr {
    pub fn new(attack_s: f32, decay_s: f32, sustain: f32, release_s: f32) -> Self {
        Self {
            attack_s,
            decay_s,
            sustain,
            release_s,
        }
    }
}

pub type AdsrHandle = Shared<Adsr>;

#[inline]
pub fn adsr_handle(adsr: Adsr) -> AdsrHandle {
    Shared::new(adsr)
}

#[inline]
pub fn adsr(input: SynthSource, adsr: AdsrHandle, gate: Gate) -> SynthSource {
    let sr = input.sample_rate().max(1);
    Box::new(AdsrSource::new(input, adsr, gate, sr))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Attack,
    Decay,
    Sustain,
    Release,
    Done,
}

struct AdsrSource {
    input: SynthSource,
    adsr: AdsrHandle,
    gate: Gate,
    sample_rate: u32,
    stage: Stage,
    amp: f32,
    release_step: f32,
}

impl AdsrSource {
    fn new(input: SynthSource, adsr: AdsrHandle, gate: Gate, sample_rate: u32) -> Self {
        Self {
            input,
            adsr,
            gate,
            sample_rate,
            stage: Stage::Attack,
            amp: 0.0,
            release_step: 0.0,
        }
    }

    fn step(&mut self) -> f32 {
        let a = self.adsr.get();
        let sr = self.sample_rate as f32;

        let sustain = a.sustain.clamp(0.0, 1.0);
        let attack_step = 1.0 / (a.attack_s.max(0.0) * sr).max(1.0);
        let decay_step = (1.0 - sustain) / (a.decay_s.max(0.0) * sr).max(1.0);

        if !self.gate.load(Ordering::Relaxed)
            && self.stage != Stage::Release
            && self.stage != Stage::Done
        {
            self.stage = Stage::Release;
            let release_samples = (a.release_s.max(0.0) * sr).max(1.0);
            self.release_step = self.amp / release_samples;
        }

        match self.stage {
            Stage::Attack => {
                self.amp += attack_step;
                if self.amp >= 1.0 {
                    self.amp = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                self.amp -= decay_step;
                if self.amp <= sustain {
                    self.amp = sustain;
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Sustain => {
                self.amp = sustain;
            }
            Stage::Release => {
                self.amp -= self.release_step;
                if self.amp <= 0.0 {
                    self.amp = 0.0;
                    self.stage = Stage::Done;
                }
            }
            Stage::Done => {
                self.amp = 0.0;
            }
        }

        self.amp
    }
}

impl Iterator for AdsrSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stage == Stage::Done {
            return None;
        }

        let x = self.input.next()?;
        let env = self.step();

        if self.stage == Stage::Done {
            return None;
        }

        Some(x * env)
    }
}

impl Source for AdsrSource {
    fn current_span_len(&self) -> Option<usize> {
        self.input.current_span_len()
    }

    fn channels(&self) -> u16 {
        self.input.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
