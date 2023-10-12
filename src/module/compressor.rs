use super::{common::*, ModuleKey, NULL_KEY, ModuleManager};

const MICROSECONDS_PER_SECOND: f32 = 1_000_000.0;

pub struct Compressor {
    signal_in_key: ModuleKey,
    slew_time: f32, // microseconds
    compression_factor: f32,
    over_compression: f32, // A boost to the initial compression factor
}

impl Compressor {
    pub fn new() -> Self {
        let signal_in_key = NULL_KEY;
        let slew_time = MICROSECONDS_PER_SECOND;
        let compression_factor = 1.0;
        let over_compression = 0.1;
        Compressor { signal_in_key, slew_time, compression_factor, over_compression }
    }

    pub fn set_signal_in(&mut self, input: ModuleKey) {
        self.signal_in_key = input;
    }

    pub fn set_slew_time(&mut self, slew_time: f32) {
        self.slew_time = slew_time;
    }

    pub fn set_over_compression(&mut self, over_compression: f32) {
        self.over_compression = over_compression;
    }
}

impl SignalOutputModule for Compressor {
    fn fill_output_buffer(&mut self, buffer: &mut [f32], output_info: &OutputInfo, manager: &mut ModuleManager) {
        let buffer_len = buffer.len();

        // Get signal from input
        let mut signal = vec![0.0; buffer_len];
        if let Some(signal_in) = manager.get_mut(self.signal_in_key) {
            signal_in.fill_output_buffer(&mut signal, output_info, manager)
        }
        else {
            buffer.fill(0.0);
            return;
        }

        let mut compression_factor = self.compression_factor;
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
        self.compression_factor = compression_factor;
    }
}