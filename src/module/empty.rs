use super::traits::{SignalOutputModule, OutputInfo};

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