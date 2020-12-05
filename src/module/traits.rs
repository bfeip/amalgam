/// Trait for modules that output a signal of some kind, audio or control
pub trait SignalOutputModule: std::marker::Send {
    /// Fills a provided buffer with the signal output
    fn fill_output_buffer(&mut self, buffer: &mut [f32]);
}