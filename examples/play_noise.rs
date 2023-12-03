use amalgam::error::*;
use amalgam::Synth;

use std::rc::Rc;

fn main() -> SynthResult<()> {
    let mut synth = match Synth::new() {
        Ok(synth) => synth,
        Err(err) => {
            let msg = format!("Failed to test full synth: {}", err);
            return Err(SynthError::new(&msg));
        }
    };

    let noise = Rc::new(amalgam::module::NoiseGenerator::new());
    synth.get_output_module_mut().set_audio_input(Some(noise));

    synth.play()?;

    loop {
        synth.gen_samples()?;
    }
}