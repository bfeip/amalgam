use std::rc::Rc;
use std::cell::Cell;

use super::common::*;

const MICROSECONDS_PER_SECOND: f32 = 1_000_000.0;

pub struct Compressor {
    signal_in: Option<Rc<dyn SynthModule>>,
    slew_time: f32, // microseconds
    compression_factor: Cell<f32>,
    over_compression: f32, // A boost to the initial compression factor
}

impl Compressor {
    pub fn new() -> Self {
        let signal_in = None;
        let slew_time = MICROSECONDS_PER_SECOND;
        let compression_factor = Cell::new(1.0);
        let over_compression = 0.1;
        Compressor { signal_in, slew_time, compression_factor, over_compression }
    }

    pub fn set_signal_in(&mut self, input: Option<Rc<dyn SynthModule>>) {
        self.signal_in = input;
    }

    pub fn set_slew_time(&mut self, slew_time: f32) {
        self.slew_time = slew_time;
    }

    pub fn set_over_compression(&mut self, over_compression: f32) {
        self.over_compression = over_compression;
    }
}

impl SynthModule for Compressor {
    fn fill_output_buffer(&self, buffer: &mut [f32], output_info: &OutputInfo) {
        let buffer_len = buffer.len();

        // Get signal from input
        let mut signal = vec![0.0; buffer_len];
        if let Some(signal_in) = &self.signal_in {
            signal_in.fill_output_buffer(&mut signal, output_info)
        }
        else {
            buffer.fill(0.0);
            return;
        }

        let mut compression_factor = self.compression_factor.get();
        for i in 0..buffer_len {
            if signal[i] < 1.0 {
                // Set a new compression factor if we need to
                let new_compression_factor = signal[i] + self.over_compression;
                compression_factor = compression_factor.max(new_compression_factor);
            }
            else {
                let microseconds_per_sample = MICROSECONDS_PER_SECOND / output_info.sample_rate as f32;
                let compression_decrease = microseconds_per_sample / self.slew_time;
                compression_factor -= compression_decrease;
            }
            buffer[i] = signal[i] / compression_factor
        }
        self.compression_factor.set(compression_factor);
    }
}