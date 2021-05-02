pub struct OutputInfo {
    pub sample_rate: usize,
    pub current_sample_range: Vec<usize>
}

impl OutputInfo {
    pub fn new(sample_rate: usize, current_sample_range: Vec<usize>) -> Self {
        OutputInfo { sample_rate, current_sample_range }
    }
}

/// Trait for modules that output a signal of some kind, audio or control
pub trait SignalOutputModule: std::marker::Send {
    /// Fills a provided buffer with the signal output
    fn fill_output_buffer(&mut self, buffer: &mut [f32], output_info: &OutputInfo);
}