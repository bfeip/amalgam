use synth::error::*;

fn main() -> SynthResult<()> {
    let mut synth = match synth::Synth::new() {
        Ok(synth) => synth,
        Err(err) => {
            let msg = format!("Failed to test full synth: {}", err);
            return Err(SynthError::new(&msg));
        }
    };

    let mut audio_output = match synth::output::AudioOutput::new(synth::output::OutputDeviceType::Cpal) {
        Ok(audio_output) => audio_output,
        Err(err) => {
            let msg = format!("Failed to test output: failed to create audio output: {}", err);
            return Err(SynthError::new(&msg));
        }
    };

    let oscillator = Box::new(synth::module::oscillator::Oscillator::new());
    synth.get_output_module_mut().set_audio_input(oscillator);
    let synth_mutex_ptr = std::sync::Arc::new(std::sync::Mutex::new(synth));
    let synth_mutex_ptr_clone = synth_mutex_ptr.clone();

    if let Err(err) = synth::Synth::play(synth_mutex_ptr, &mut audio_output) {
        let msg = format!("Failed to test full synth: {}", err);
        return Err(SynthError::new(&msg));
    }

    // The audio is being played on a separate thread owned by cpal if I understand correctly
    // so we need to sleep here to give it enough time to play a bit
    std::thread::sleep(std::time::Duration::from_secs(2));

    match synth_mutex_ptr_clone.lock() {
        Ok(locked_synth) => {
            println!("{:?}", locked_synth.debug_sample_buffer);
        }
        Err(_err) => {
            // somethings fucked up
        }
    }

    Ok(())
}