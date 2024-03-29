use super::{SynthModule, OutputInfo};

pub struct SampleBuffer {
    samples: Vec<f32>,
}

impl SampleBuffer {
    pub fn new(samples: Vec<f32>) -> Self {
        Self { samples }
    }
}

impl SynthModule for SampleBuffer {
    fn fill_output_buffer(&self, buffer: &mut [f32], _output_info: &OutputInfo) {
        debug_assert!(buffer.len() == self.samples.len());
        for (output_sample, stored_sample) in buffer.iter_mut().zip(self.samples.iter()) {
            *output_sample = *stored_sample;
        }
    }
}
