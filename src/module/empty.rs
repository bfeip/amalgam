use super::common::*;
use crate::note::Note;

use std::collections::HashSet;

/// A zero sized struct representing a module that outputs nothing
pub struct Empty;

impl Empty {
    /// Creates and returns a new Empty module e.g. does nothing
    pub fn new() -> Self {
        Empty {}
    }
}

impl SignalOutputModule for Empty {
    fn fill_output_buffer(&mut self, data: &mut [f32], _output_info: &OutputInfo) {
        for datum in data.iter_mut() {
            *datum = 0_f32;
        }
    }
}

impl NoteOutputModule for Empty {
    fn get_output(&mut self, n_samples: usize, _output_info: &OutputInfo) -> Vec<HashSet<Note>> {
        vec![HashSet::new(); n_samples]
    }

    fn fill_output_buffer(&mut self, buffer: &mut [HashSet<Note>], _output_info: &OutputInfo) {
        for notes in buffer.iter_mut() {
            *notes = HashSet::new();
        }
    }
}

pub struct OptionalEmpty;

impl OptionalEmpty {
    pub fn new() -> Self {
        Self {}
    }
}

impl OptionalSignalOutputModule for OptionalEmpty {
    fn fill_optional_output_buffer(&mut self, buffer: &mut[Option<f32>], _output_info: &OutputInfo) {
        for datum in buffer.iter_mut() {
            *datum = None;
        }
    }
}