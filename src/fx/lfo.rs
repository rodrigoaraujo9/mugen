use std::f32::consts::TAU;

use crate::patches::basic::BasicKind;

#[derive(Clone)]
pub struct LfoOsc {
    kind: BasicKind,
    phase: f32,     // [0, 1)
    phase_inc: f32, // cycles per sample
    rng: u64,       // only used for Noise
}

impl LfoOsc {
    pub fn new(kind: BasicKind, rate_hz: f32, sr: u32) -> Self {
        let sr_f = sr.max(1) as f32;
        let rate = rate_hz.max(0.0);
        Self {
            kind,
            phase: 0.0,
            phase_inc: rate / sr_f,
            rng: 0x1234_5678_9ABC_DEF0,
        }
    }

    #[inline]
    fn step_phase(&mut self) -> f32 {
        let p = self.phase;
        self.phase += self.phase_inc;
        // [0, 1)
        if self.phase >= 1.0 {
            self.phase -= self.phase.floor();
        }
        p
    }

    #[inline]
    fn next_noise(&mut self) -> f32 {
        // xorshift64*
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

    /// returns LFO value in [-1, +1]
    #[inline]
    pub fn next(&mut self) -> f32 {
        match self.kind {
            BasicKind::Sine => {
                let p = self.step_phase();
                (TAU * p).sin()
            }
            BasicKind::Square => {
                let p = self.step_phase();
                if p < 0.5 { 1.0 } else { -1.0 }
            }
            BasicKind::Triangle => {
                let p = self.step_phase();
                // [-1,1]
                if p < 0.5 {
                    -1.0 + 4.0 * p
                } else {
                    3.0 - 4.0 * p
                }
            }
            BasicKind::Saw => {
                let p = self.step_phase();
                // [-1,1]
                2.0 * p - 1.0
            }
            BasicKind::Noise => self.next_noise(),
        }
    }
}
