use std::sync::{Arc, Mutex};
use std::collections::HashSet;

use crate::note::Note;

pub type MutexPtr<T> = Arc<Mutex<T>>;

pub trait IntoMutexPtr: Sized {
    fn into_mutex_ptr(self) -> MutexPtr<Self>;
}

impl<T> IntoMutexPtr for T {
    fn into_mutex_ptr(self) -> MutexPtr<Self> {
        Arc::new(Mutex::new(self))
    }
}

#[derive(Copy, Clone)]
pub enum EdgeDetection {
    Rising,
    Falling,
    Both
}

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
pub trait SignalOutputModule: Send {
    /// Fills a provided buffer with the signal output
    fn fill_output_buffer(&mut self, buffer: &mut [f32], output_info: &OutputInfo);
}

pub trait OptionalSignalOutputModule: Send {
    fn fill_optional_output_buffer(&mut self, buffer: &mut[Option<f32>], output_info: &OutputInfo);
}

impl<T: SignalOutputModule> OptionalSignalOutputModule for T {
    fn fill_optional_output_buffer(&mut self, buffer: &mut[Option<f32>], output_info: &OutputInfo) {
        let buffer_len = buffer.len();
        let mut sample_buffer = vec![0.0; buffer_len];
        self.fill_output_buffer(sample_buffer.as_mut_slice(), output_info);
        for (&raw_sample, sample_option) in sample_buffer.iter().zip(buffer.iter_mut()) {
            *sample_option = Some(raw_sample);
        }
    }
}

pub trait NoteOutputModule: Send {
    fn get_output(&mut self, n_samples: usize, output_info: &OutputInfo) -> Vec<HashSet<Note>>;
    fn fill_output_buffer(&mut self, buffer: &mut [HashSet<Note>], output_info: &OutputInfo);
}