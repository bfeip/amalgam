use super::error::{SynthResult, SynthError};
use super::module::output::Output;
use super::module::common::{SignalOutputModule, OutputInfo, OutputTimestamp};
use super::output::{AudioOutput};
use super::clock;

use std::sync::{Arc, Mutex};

pub struct Synth {
    output_module: Output,
    sample_rate: usize,
    master_sample_clock: Option<clock::SampleClock>
}

impl Synth {
    pub fn new() -> SynthResult<Self> {
        let output_module = Output::new();
        let master_sample_clock = None;
        let sample_rate = 0;

        let synth = Synth { output_module, sample_rate, master_sample_clock };
        Ok(synth)
    }

    pub fn get_output_module(&self) -> &Output {
        &self.output_module
    }

    pub fn get_output_module_mut(&mut self) -> &mut Output {
        &mut self.output_module
    }

    pub fn play(synth: Arc<Mutex<Self>>, audio_output: &mut AudioOutput) -> SynthResult<()> {
        let sample_type = match audio_output.get_sample_format() {
            Some(sample_type) => sample_type,
            None => {
                let msg = "Tried to play from Synth that has un-configured audio output";
                return Err(SynthError::new(msg));
            }
        };

        // Get and store the sample rate
        let new_sample_rate = match audio_output.get_sample_rate() {
            Some(sample_rate) => sample_rate.0 as usize,
            None => {
                let msg = "Failed to get sample rate while trying to play synth";
                return Err(SynthError::new(msg));
            }
        };

        // Mutex block so that synth is unlocked at the end of this block
        {
            // Lock synth mutex and so we can start doing things with it
            let mut locked_synth = match synth.lock() {
                Ok(locked_synth) => locked_synth,
                Err(err) => {
                    let msg = format!("Can't play. Synth lock is poisoned!: {}", err);
                    return Err(SynthError::new(&msg));
                }
            };

            let sample_rate_has_changed = locked_synth.sample_rate != new_sample_rate;
            locked_synth.sample_rate = new_sample_rate;

            // If the clock has not yet been initialized or it's invalid because the sample rate has changed
            // set up a new clock
            if locked_synth.master_sample_clock.is_none() || sample_rate_has_changed {
                let clock = clock::SampleClock::new(new_sample_rate);
                locked_synth.master_sample_clock = Some(clock);
            }
        }

        match sample_type {
            cpal::SampleFormat::F32 => Self::play_with_cpal::<f32>(synth, audio_output),
            cpal::SampleFormat::I16 => Self::play_with_cpal::<i16>(synth, audio_output),
            cpal::SampleFormat::U16 => Self::play_with_cpal::<u16>(synth, audio_output)
        }
    }

    fn play_with_cpal<T: cpal::Sample>(synth: Arc<Mutex<Self>>, audio_output: &mut AudioOutput) -> SynthResult<()> {
        let channel_count = audio_output.get_channel_count().unwrap();

        // Create a callback to pass to CPAL to output the audio. This'll get passed to a different thread that
        // will actually play the audio.
        let output_callback = move |sample_buffer: &mut [T], callback_info: &cpal::OutputCallbackInfo| {
            // Lock synth mutex and so we can start doing things with it
            let mut locked_synth = match synth.lock() {
                Ok(locked_synth) => locked_synth,
                Err(err) => {
                    // We're in trouble here. Since this is on the audio output thread all I can really do
                    // is panic and hope CPAL handles it more gracefully than just blowing up
                    panic!("Failed in audio output. Synth lock is poisoned!: {}", err);
                }
            };

            let sample_rate = locked_synth.sample_rate;
            let mut sample_clock = locked_synth.master_sample_clock.unwrap();
            let output_module = &mut locked_synth.output_module;

            let buffer_length = sample_buffer.len();
            let mut f32_buffer = Vec::with_capacity(buffer_length);
            for _ in 0..buffer_length {
                f32_buffer.push(0_f32);
            }

            let timestamp = OutputTimestamp::new(callback_info.timestamp());
            let clock_values = sample_clock.get_range(buffer_length);
            let output_info = OutputInfo::new(sample_rate, channel_count, clock_values, timestamp);

            output_module.fill_output_buffer(&mut f32_buffer, &output_info);
            for i in 0..buffer_length {
                sample_buffer[i] = T::from(&f32_buffer[i]);
            }
        };

        if let Err(err) = audio_output.set_output_callback(output_callback) {
            let msg = format!("Failed to play from Synth because we couldn't set the output callback: {}", err);
            return Err(SynthError::new(&msg));
        }

        if let Err(err) = audio_output.play() {
            let msg = format!("Failed to play audio stream: {}", err);
            return Err(SynthError::new(&msg))
        }

        Ok(())
    }
}