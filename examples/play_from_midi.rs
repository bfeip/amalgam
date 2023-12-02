use amalgam::error::*;
use amalgam::Synth;
use amalgam::module;

use std::rc::Rc;

const MIDI_PATH: &'static str = "data/basic_test.mid";

fn main() -> SynthResult<()> {
    let mut synth = match Synth::new() {
        Ok(synth) => synth,
        Err(err) => {
            let msg = format!("Failed to test full synth: {}", err);
            return Err(SynthError::new(&msg));
        }
    };

    let midi = Rc::new(module::MidiModuleBase::open(MIDI_PATH)?);
    let midi_note = Rc::new(module::MidiNoteOutput::new(midi));

    let mut oscillator = module::Oscillator::new();
    oscillator.set_exponential_freq_input(Some(midi_note));
    synth.get_output_module_mut().set_audio_input(Some(Rc::new(oscillator)));

    synth.play()?;

    loop {
        synth.gen_samples()?;
    }
}