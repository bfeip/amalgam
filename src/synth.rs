use super::error::{SynthResult, SynthError};
use super::module::output::Output;
use super::module::traits::SignalOutputModule;
use super::output::{AudioOutput};

pub struct Synth {
    output_module: Output,
}

impl Synth {
    pub fn new() -> SynthResult<Self> {
        let output_module = Output::new();

        let synth = Synth { output_module };
        Ok(synth)
    }

    pub fn get_output_module(&self) -> &Output {
        &self.output_module
    }

    pub fn get_output_module_mut(&mut self) -> &mut Output {
        &mut self.output_module
    }

    pub fn play(self, audio_output: &mut AudioOutput) -> SynthResult<()> {
        let sample_type = match audio_output.get_sample_format() {
            Some(sample_type) => sample_type,
            None => {
                let msg = "Tried to play from Synth that has un-configured audio output";
                return Err(SynthError::new(msg));
            }
        };

        match sample_type {
            cpal::SampleFormat::F32 => self.play_helper::<f32>(audio_output),
            cpal::SampleFormat::I16 => self.play_helper::<i16>(audio_output),
            cpal::SampleFormat::U16 => self.play_helper::<u16>(audio_output)
        }
    }

    pub fn play_helper<T: cpal::Sample>(self, audio_output: &mut AudioOutput) -> SynthResult<()> {
        let mut output_module = self.output_module;

        let output_callback = move |sample_buffer: &mut [T], _: &cpal::OutputCallbackInfo| {
            let buffer_length = sample_buffer.len();
            let mut f32_buffer = Vec::with_capacity(buffer_length);
            for _ in 0..buffer_length {
                f32_buffer.push(0_f32);
            }
            output_module.fill_output_buffer(&mut f32_buffer);
            for i in 0..buffer_length {
                unsafe {
                    // Note: unchecked if fine here since we're using sample_buffer's length and
                    // the f32 buffer is the same size
                    *sample_buffer.get_unchecked_mut(i) = T::from(f32_buffer.get_unchecked(i));
                }
            }
        };

        if let Err(err) = audio_output.set_output_callback(output_callback) {
            let msg = format!("Failed to play from Synth because we couldn't set the output callback: {}", err);
            return Err(SynthError::new(&msg));
        }

        if let Err(err) = audio_output.play() {
            let msg = format!("Failed to play audio stream: {}", err);
            Err(SynthError::new(&msg))
        } else {
            Ok(())
        }
    }
}