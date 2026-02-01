use rodio::Source;
use rodio::source::{SawtoothWave, SineWave};
use std::time::Duration;
use crate::config::AMP_DEFAULT;

pub trait AudioSource: Send + Sync {
    fn create_source(&self, frequency: f32) -> Box<dyn Source<Item = f32> + Send>;
    fn name(&self) -> &'static str;
}

pub struct WaveSource {
    pub amplitude: f32,
    pub duration: Duration,
}

impl WaveSource {
    pub fn new(amplitude: f32) -> Self {
        Self {
            amplitude,
            duration: Duration::from_secs(3600),
        }
    }
}

impl Default for WaveSource {
    fn default() -> Self {
        Self::new(AMP_DEFAULT)
    }
}

impl AudioSource for WaveSource {
    fn create_source(&self, frequency: f32) -> Box<dyn Source<Item = f32> + Send> {
        Box::new(
            SineWave::new(frequency)
                .take_duration(self.duration)
                .amplify(self.amplitude)
        )
    }

    fn name(&self) -> &'static str {
        "Sine Wave"
    }
}
