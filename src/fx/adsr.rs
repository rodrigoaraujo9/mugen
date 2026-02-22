use rodio::Source;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use crate::audio_patch::Node;

pub type SynthSource = Box<dyn Source<Item = f32> + Send>;
pub type Gate = Arc<AtomicBool>;

#[derive(Clone, Copy, Debug)]
pub struct Adsr {
    pub attack_s: f32, //sec
    pub decay_s: f32, //sec
    pub sustain: f32, //0..1
    pub release_s: f32, //sec
}

#[derive(Clone, Copy, Debug)]
pub struct AdsrEnvelope {
    pub sustain: f32,
    pub attack_step: f32,
    pub decay_step: f32,
    pub release_step: f32,
    pub release_samples: f32, // needed to calc release samples (given current amp early release)
}

impl Adsr {
    pub fn new(attack_s: f32, decay_s: f32, sustain: f32, release_s: f32) -> Self {
        Self { attack_s, decay_s, sustain, release_s }
    }

    pub fn to_envelope(&self, sample_rate: u32) -> AdsrEnvelope {
        let sr = sample_rate as f32;

        let attack_samples = (self.attack_s.max(0.0) * sr).max(1.0);
        let decay_samples = (self.decay_s.max(0.0) * sr).max(1.0);
        let release_samples = (self.release_s.max(0.0) * sr).max(1.0);

        let sustain = self.sustain.clamp(0.0, 1.0);

        AdsrEnvelope {
            sustain,
            attack_step: 1.0 / attack_samples,
            decay_step: (1.0 - sustain) / decay_samples,
            release_step: sustain / release_samples,
            release_samples,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage { Attack, Decay, Sustain, Release, Done }

pub struct AdsrNode {
    pub adsr: Adsr,
    pub sample_rate: u32,
    pub gate: Gate,
}

impl AdsrNode {
    pub fn new(adsr: Adsr, sample_rate: u32, gate: Gate) -> Self {
        Self { adsr, sample_rate, gate }
    }
}

pub struct AdsrSource {
    input: SynthSource,
    envelope: AdsrEnvelope,
    gate: Gate,
    sample_rate: u32,
    stage: Stage,
    current_amp: f32,
}

impl AdsrSource {
    pub fn new(input: SynthSource, adsr: Adsr, sample_rate: u32, gate: Gate) -> Self {
        Self {
            input,
            envelope: adsr.to_envelope(sample_rate),
            gate,
            sample_rate,
            stage: Stage::Attack,
            current_amp: 0.0,
        }
    }

    fn step_envelope(&mut self) -> f32 {
        if !self.gate.load(Ordering::Relaxed)
            && self.stage != Stage::Release
            && self.stage != Stage::Done
        {
            self.stage = Stage::Release;
            self.envelope.release_step=self.current_amp/self.envelope.release_samples;
        }

        match self.stage {
            Stage::Attack => {
                self.current_amp += self.envelope.attack_step;
                if self.current_amp >= 1.0 {
                    self.current_amp = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                self.current_amp -= self.envelope.decay_step;
                if self.current_amp <= self.envelope.sustain {
                    self.current_amp = self.envelope.sustain;
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Sustain => {
                self.current_amp = self.envelope.sustain;
            }
            Stage::Release => {
                self.current_amp -= self.envelope.release_step;
                if self.current_amp <= 0.0 {
                    self.current_amp = 0.0;
                    self.stage = Stage::Done;
                }
            }
            Stage::Done => {
                self.current_amp = 0.0;
            }
        }

        self.current_amp
    }
}

impl Iterator for AdsrSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.stage == Stage::Done {
            return None;
        }

        let x = self.input.next()?;
        let env = self.step_envelope();

        if self.stage == Stage::Done {
            return None;
        }

        Some(x * env)
    }
}

impl Source for AdsrSource {
    fn current_span_len(&self) -> Option<usize> { self.input.current_span_len() }
    fn channels(&self) -> u16 { self.input.channels() }
    fn sample_rate(&self) -> u32 { self.sample_rate }
    fn total_duration(&self) -> Option<Duration> { None }
}

impl Node for AdsrNode {
    fn apply(&self, input: SynthSource) -> SynthSource {
        Box::new(AdsrSource::new(input, self.adsr, self.sample_rate, self.gate.clone()))
    }
    fn name(&self) -> &'static str { "ADSR" }
}
