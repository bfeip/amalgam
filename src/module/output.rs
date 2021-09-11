use super::common::{SignalOutputModule, OutputInfo};
use super::empty::Empty;

/// A structure representing controls that would typically be on a output module
/// of a modular synth.
pub struct Output {
    volume: f32,
    panning: f32,
    audio_input: Box<dyn SignalOutputModule>,
}

impl Output {
    pub fn new() -> Self {
        let volume = 1.0;
        let panning = 0.5;
        let audio_input = Box::new(Empty::new());

        Self { volume, panning, audio_input }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume;
    }

    pub fn set_panning(&mut self, panning: f32) {
        self.panning = panning;
    }

    pub fn set_audio_input(&mut self, audio_input: Box<dyn SignalOutputModule>) {
        self.audio_input = audio_input;
    }
}

impl SignalOutputModule for Output {
    fn fill_output_buffer(&mut self, data: &mut [f32], output_info: &OutputInfo) {
        let channel_count_usize = output_info.channel_count as usize;
        let total_buffer_len = data.len();
        debug_assert!(
            total_buffer_len % channel_count_usize == 0,
            "Expected buffer length to have same number of slots for each channel"
        );
        // We will take just one channel's samples and multiplex them to do the panning
        // NOTE: this assumes we have no modules that operate on stereo signals
        let mono_channel_len = total_buffer_len / channel_count_usize;
        let mut mono_channel_buffer = vec![0.0; mono_channel_len];

        // Get the audio for the one channel
        self.audio_input.fill_output_buffer(&mut mono_channel_buffer, output_info);

        // fill the final buffer with multi-channel data
        let output_chunk_iter = data.chunks_mut(channel_count_usize);
        let input_sample_iter = mono_channel_buffer.iter();
        for (output_chunk, input_sample) in output_chunk_iter.zip(input_sample_iter) {
            // TODO: panning
            for output_sample in output_chunk.iter_mut() {
                *output_sample = input_sample * self.volume;
            }
        }
    }
}