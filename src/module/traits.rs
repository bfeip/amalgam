use std::sync::{Arc, Mutex};

pub type MutexPtr<T> = Arc<Mutex<T>>;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct OutputTimestamp {
    timestamp: Option<cpal::OutputStreamTimestamp>
}

impl OutputTimestamp {
    pub fn new(timestamp: cpal::OutputStreamTimestamp) -> Self {
        Self { timestamp: Some(timestamp) }
    }

    pub fn empty() -> Self {
        Self { timestamp: None }
    }
}

pub struct OutputInfo {
    pub sample_rate: usize,
    pub current_sample_range: Vec<usize>,
    pub timestamp: OutputTimestamp
}

impl OutputInfo {
    pub fn new(sample_rate: usize, current_sample_range: Vec<usize>, timestamp: OutputTimestamp) -> Self {
        OutputInfo { sample_rate, current_sample_range, timestamp }
    }
}

/// Trait for modules that output a signal of some kind, audio or control
pub trait SignalOutputModule: std::marker::Send {
    /// Fills a provided buffer with the signal output
    fn fill_output_buffer(&mut self, buffer: &mut [f32], output_info: &OutputInfo);
}