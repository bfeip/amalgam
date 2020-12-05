#![allow(dead_code)]

extern crate cpal;

mod error;
mod prelude;
mod note;
mod clock;
mod module;
mod output;
mod synth;

use crate::error::{SynthError, SynthResult};
use crate::module::{oscillator};

/// Outputs a second of audio who's source depends on the `test_output_type` you provided
fn test_output() -> SynthResult<()> {
    let mut synth = match synth::Synth::new() {
        Ok(synth) => synth,
        Err(err) => {
            let msg = format!("Failed to test full synth: {}", err);
            return Err(SynthError::new(&msg));
        }
    };

    let mut audio_output = match output::AudioOutput::new(output::OutputDeviceType::Cpal) {
        Ok(audio_output) => audio_output,
        Err(err) => {
            let msg = format!("Failed to test output: failed to create audio output: {}", err);
            return Err(SynthError::new(&msg));
        }
    };
    let sample_rate = audio_output.get_sample_rate().unwrap();

    let oscillator = Box::new(oscillator::Oscillator::new(sample_rate.0 as f32));
    synth.get_output_module_mut().set_audio_input(oscillator);
    if let Err(err) = synth.play(&mut audio_output) {
        let msg = format!("Failed to test full synth: {}", err);
        return Err(SynthError::new(&msg));
    }

    // The audio is being played on a separate thread owned by cpal if I understand correctly
    // so we need to sleep here to give it enough time to play a bit
    std::thread::sleep(std::time::Duration::from_secs(1));

    Ok(())
}

fn main() -> SynthResult<()> {
    test_output()
}
