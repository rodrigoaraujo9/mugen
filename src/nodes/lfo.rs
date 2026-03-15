use crate::generators::basic::BasicKind;
use std::f32::consts::TAU;

#[derive(Clone)]
pub struct LfoOsc {
    kind: BasicKind,
    phase: f32,
    rate_hz: f32,
    sample_rate: u32,
    phase_inc: f32,
    rng: u64,
}

impl LfoOsc {
    pub fn new(kind: BasicKind, rate_hz: f32, sample_rate: u32) -> Self {
        let mut osc = Self {
            kind,
            phase: 0.0,
            rate_hz: rate_hz.max(0.0),
            sample_rate: sample_rate.max(1),
            phase_inc: 0.0,
            rng: 0x1234_5678_9ABC_DEF0,
        };
        osc.recalc();
        osc
    }

    fn recalc(&mut self) {
        self.phase_inc = self.rate_hz.max(0.0) / self.sample_rate.max(1) as f32;
    }

    pub fn sync_sr(&mut self, sample_rate: u32) {
        let sample_rate = sample_rate.max(1);
        if sample_rate != self.sample_rate {
            self.sample_rate = sample_rate;
            self.recalc();
        }
    }

    pub fn set_rate_hz(&mut self, rate_hz: f32) {
        let rate_hz = rate_hz.max(0.0);
        if (rate_hz - self.rate_hz).abs() > f32::EPSILON {
            self.rate_hz = rate_hz;
            self.recalc();
        }
    }

    pub fn set_kind(&mut self, kind: BasicKind) {
        self.kind = kind;
    }

    fn step_phase(&mut self) -> f32 {
        let phase = self.phase;
        self.phase += self.phase_inc;

        if self.phase >= 1.0 {
            self.phase -= self.phase.floor();
        }

        phase
    }

    fn next_noise(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;

        let y = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        let u = (y >> 40) as u32;
        let f = u as f32 / ((1u32 << 24) as f32);

        2.0 * f - 1.0
    }

    pub fn next_value(&mut self) -> f32 {
        match self.kind {
            BasicKind::Sine => (TAU * self.step_phase()).sin(),
            BasicKind::Square => {
                if self.step_phase() < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            BasicKind::Triangle => {
                let p = self.step_phase();
                if p < 0.5 {
                    -1.0 + 4.0 * p
                } else {
                    3.0 - 4.0 * p
                }
            }
            BasicKind::Saw => 2.0 * self.step_phase() - 1.0,
            BasicKind::Noise => self.next_noise(),
        }
    }
}
