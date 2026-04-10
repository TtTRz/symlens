/// A simple audio engine.
pub struct AudioEngine {
    pub sample_rate: u32,
    pub buffer_size: usize,
}

impl AudioEngine {
    /// Create a new audio engine.
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            buffer_size: 1024,
        }
    }

    /// Process an audio block.
    pub fn process_block(&mut self, data: &mut [f32]) {
        for sample in data.iter_mut() {
            *sample = normalize(*sample);
        }
    }
}

/// Normalize a sample to [-1.0, 1.0].
fn normalize(sample: f32) -> f32 {
    sample.clamp(-1.0, 1.0)
}

pub const MAX_CHANNELS: usize = 8;

pub type SampleRate = u32;

pub enum AudioFormat {
    Mono,
    Stereo,
    Surround,
}

pub trait Processor {
    fn process(&mut self, input: &[f32]) -> Vec<f32>;
}
