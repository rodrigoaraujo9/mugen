use std::sync::{Arc, Mutex};
use std::time::Instant;
use rodio::Source;

pub type Matrix<T> = Vec<Vec<T>>;

pub struct AudioCapture {
    buffer: Arc<Mutex<CaptureBuffer>>,
}

struct CaptureBuffer {
    data: Matrix<f64>,
    sample_rate: u32,
    last_update: Instant,
}

impl AudioCapture {
    pub fn new(channels: usize, buffer_size: usize, sample_rate: u32) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(CaptureBuffer {
                data: vec![vec![0.0; buffer_size]; channels],
                sample_rate,
                last_update: Instant::now(),
            })),
        }
    }

    pub fn get_data(&self) -> Option<Matrix<f64>> {
        self.buffer.lock().ok().map(|buf| buf.data.clone())
    }

    pub fn get_sample_rate(&self) -> u32 {
        self.buffer.lock().ok().map(|buf| buf.sample_rate).unwrap_or(48000)
    }

    pub fn create_tap_source<S>(&self, source: S, channels: usize) -> TapSource<S>
    where
        S: Source<Item = f32>,
    {
        TapSource {
            source,
            buffer: Arc::clone(&self.buffer),
            channels,
            sample_buffer: Vec::new(),
        }
    }
}

pub struct TapSource<S> {
    source: S,
    buffer: Arc<Mutex<CaptureBuffer>>,
    channels: usize,
    sample_buffer: Vec<f32>,
}

impl<S> Iterator for TapSource<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.source.next()?;
        self.sample_buffer.push(sample);

        let buffer_size = {
            let buf = self.buffer.lock().ok()?;
            buf.data.first()?.len()
        };

        if self.sample_buffer.len() >= buffer_size * self.channels {
            if let Ok(mut buf) = self.buffer.lock() {
                buf.data = stream_to_matrix(
                    self.sample_buffer.iter().copied(),
                    self.channels,
                    1.0,
                );
                buf.last_update = Instant::now();
                self.sample_buffer.clear();
            }
        }

        Some(sample)
    }
}

impl<S> Source for TapSource<S>
where
    S: Source<Item = f32>,
{
    fn current_span_len(&self) -> Option<usize> {
        self.source.current_span_len()
    }

    fn channels(&self) -> u16 {
        self.source.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.source.sample_rate()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        self.source.total_duration()
    }
}

fn stream_to_matrix<I>(
    stream: impl Iterator<Item = I>,
    channels: usize,
    norm: f64,
) -> Matrix<f64>
where
    I: Copy + Into<f64>,
{
    let mut out = vec![vec![]; channels];
    let mut channel = 0;
    for sample in stream {
        let normalized: f64 = sample.into() / norm;
        out[channel].push(normalized);
        channel = (channel + 1) % channels;
    }
    out
}
