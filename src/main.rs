#![allow(dead_code)]
#![warn(missing_debug_implementations)]

extern crate cpal;

mod error;
mod util;
mod prelude;

mod midi;
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

    let oscillator = Box::new(oscillator::Oscillator::new());
    synth.get_output_module_mut().set_audio_input(oscillator);
    let synth_mutex_ptr = std::sync::Arc::new(std::sync::Mutex::new(synth));

    if let Err(err) = synth::Synth::play(synth_mutex_ptr, &mut audio_output) {
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
