mod sample_buffer;
mod compressor;
mod attenuverter;
mod noise;
mod oscillator;
mod sequencer;
mod mixer;
mod envelope;
mod midi;
mod output;
//mod voice;

pub use compressor::Compressor;
pub use attenuverter::Attenuverter;
pub use noise::NoiseGenerator;
pub use oscillator::Oscillator;
pub use sequencer::Sequencer;
pub use mixer::Mixer;
pub use envelope::Envelope;
pub use midi::MidiModuleBase;
//pub use midi::midi_note::MidiNoteOutput;
pub use output::Output;

use std::time::Instant;

use crate::clock::SampleRange;

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
        CompressionMode::None => (),
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

pub struct OutputInfo {
    pub sample_rate: usize,
    pub channel_count: u16,
    pub current_sample_range: SampleRange,
    pub timestamp: Instant
}

impl OutputInfo {
    pub fn new(
        sample_rate: usize, channel_count: u16,
        current_sample_range: SampleRange, timestamp: Instant
    ) -> Self {
        OutputInfo { sample_rate, channel_count, current_sample_range, timestamp }
    }

    #[cfg(test)]
    pub fn new_basic(sample_rate: usize, current_sample_range: SampleRange) -> Self {
        let channel_count = 1;
        let timestamp = Instant::now();
        OutputInfo { sample_rate, channel_count, current_sample_range, timestamp }
    }
}

/// Trait for modules that output a signal of some kind, audio or control
pub trait SynthModule {
    /// Fills a provided buffer with the signal output
    fn fill_output_buffer(&self, buffer: &mut [f32], output_info: &OutputInfo);
}