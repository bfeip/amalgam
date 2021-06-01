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
        self.audio_input.fill_output_buffer(data, output_info);
        for datum in data.iter_mut() {
            // TODO: panning
            *datum = *datum * self.volume;
        }
    }
}