use std::time::Duration;
use rodio::Source;
use rodio::source::{SineWave, SquareWave, TriangleWave, SawtoothWave};
use crate::patch::{Generator, SynthSource};
use crate::config::{AMP_DEFAULT, ENDLESS, SAMPLE_RATE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BasicKind {
    Sine,
    Saw,
    Square,
    Triangle,
    Noise,
}

impl BasicKind {
    pub fn toggle(self) -> Self {
        match self {
            BasicKind::Sine => BasicKind::Saw,
            BasicKind::Saw => BasicKind::Square,
            BasicKind::Square => BasicKind::Triangle,
            BasicKind::Triangle => BasicKind::Noise,
            BasicKind::Noise => BasicKind::Sine,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            BasicKind::Sine => "Sine",
            BasicKind::Saw => "Saw",
            BasicKind::Square => "Square",
            BasicKind::Triangle => "Triangle",
            BasicKind::Noise => "Noise",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct NoiseParams {
    seed: u64,
    sample_rate: u32,
}

pub fn basic_generator(kind: BasicKind) -> Box<dyn Generator> {
    let noise = if kind == BasicKind::Noise {
        Some(NoiseParams {
            seed: 0x1234_5678_9ABC_DEF0,
            sample_rate: SAMPLE_RATE,
        })
    } else {
        None
    };

    Box::new(BasicGenerator {
        kind,
        amplitude: AMP_DEFAULT,
        duration: ENDLESS,
        noise,
    })
}

struct BasicGenerator {
    kind: BasicKind,
    amplitude: f32,
    duration: Duration,
    noise: Option<NoiseParams>,
}

impl Generator for BasicGenerator {
    fn create(&self, frequency: f32) -> SynthSource {
        match self.kind {
            BasicKind::Sine => Box::new(
                SineWave::new(frequency)
                    .amplify(self.amplitude)
                    .take_duration(self.duration),
            ),

            BasicKind::Square => Box::new(
                SquareWave::new(frequency)
                    .amplify(self.amplitude)
                    .take_duration(self.duration),
            ),

            BasicKind::Triangle => Box::new(
                TriangleWave::new(frequency)
                    .amplify(self.amplitude)
                    .take_duration(self.duration),
            ),

            BasicKind::Saw => Box::new(
                SawtoothWave::new(frequency)
                    .amplify(self.amplitude)
                    .take_duration(self.duration),
            ),

            BasicKind::Noise => {
                let p = self.noise.expect("Noise params missing for BasicKind::Noise");
                Box::new(
                    NoiseGen::new(p.seed, p.sample_rate)
                        .amplify(self.amplitude)
                        .take_duration(self.duration),
                )
            }
        }
    }

    fn name(&self) -> &'static str {
        self.kind.name()
    }
}

struct NoiseGen {
    rng: u64,
    sr: u32,
}

impl NoiseGen {
    fn new(seed: u64, sr: u32) -> Self {
        Self { rng: seed, sr }
    }

    fn next_noise(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        let y = x.wrapping_mul(0x2545F4914F6CDD1D);

        let u = (y >> 40) as u32;
        let f = u as f32 / ((1u32 << 24) as f32);
        2.0 * f - 1.0
    }
}

impl Iterator for NoiseGen {
    type Item = f32;
    fn next(&mut self) -> Option<f32> { Some(self.next_noise()) }
}

impl Source for NoiseGen {
    fn current_span_len(&self) -> Option<usize> { None }
    fn channels(&self) -> u16 { 1 }
    fn sample_rate(&self) -> u32 { self.sr }
    fn total_duration(&self) -> Option<Duration> { None }
}
