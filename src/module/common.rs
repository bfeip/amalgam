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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum EdgeDetection {
    Rising,
    Falling,
    Both
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CompressionMode {
    None,
    Compress,
    Limit
}

pub fn compress_audio(data: &mut [f32], compression_mode: CompressionMode) {
    match compression_mode {
        CompressionMode::None => return,
        CompressionMode::Compress => {
            // TODO: This might be the poor man's compression. Should research into doing it proper
            // Find largest element of the buffer
            let mut largest_element = 0.0;
            for datum in data.iter() {
                let datum_abs = f32::abs(*datum);
                if datum_abs > largest_element {
                    largest_element = datum_abs;
                }
            }

            if largest_element < 1.0 {
                // If we're always below the limit then don't try to reduce
                return;
            }

            // Reduce all elements by a factor that makes the peaks 1.0 or -1.0
            let reduction_factor = largest_element;
            for datum in data.iter_mut() {
                *datum /= reduction_factor;
            }
        }
        CompressionMode::Limit => {
            for datum in data.iter_mut() {
                if *datum > 1.0 {
                    *datum = 1.0;
                } 
                else if *datum < -1.0 {
                    *datum = -1.0;
                }
            }
        }
    }
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
    pub channel_count: u16,
    pub current_sample_range: Vec<usize>,
    pub timestamp: OutputTimestamp
}

impl OutputInfo {
    pub fn new(
        sample_rate: usize, channel_count: u16,
        current_sample_range: Vec<usize>, timestamp: OutputTimestamp
    ) -> Self {
        OutputInfo { sample_rate, channel_count, current_sample_range, timestamp }
    }

    #[cfg(test)]
    pub fn new_basic(sample_rate: usize, current_sample_range: Vec<usize>) -> Self {
        let channel_count = 1;
        let timestamp = OutputTimestamp::empty();
        OutputInfo { sample_rate, channel_count, current_sample_range, timestamp }
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