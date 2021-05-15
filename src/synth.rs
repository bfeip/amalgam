use super::error::{SynthResult, SynthError};
use super::module::output::Output;
use super::module::traits::{SignalOutputModule, OutputInfo, OutputTimestamp};
use super::output::{AudioOutput};
use super::clock;

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

    pub fn play(mut self, audio_output: &mut AudioOutput) -> SynthResult<()> {
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
        let sample_rate_has_changed = self.sample_rate != new_sample_rate;
        self.sample_rate = new_sample_rate;

        // If the clock has not yet been initalized or it's invalid because the sample rate has changed
        // set up a new clock
        if self.master_sample_clock.is_none() || sample_rate_has_changed {
            let clock = clock::SampleClock::new(new_sample_rate);
            self.master_sample_clock = Some(clock);
        }

        match sample_type {
            cpal::SampleFormat::F32 => self.play_helper::<f32>(audio_output),
            cpal::SampleFormat::I16 => self.play_helper::<i16>(audio_output),
            cpal::SampleFormat::U16 => self.play_helper::<u16>(audio_output)
        }
    }

    pub fn play_helper<T: cpal::Sample>(self, audio_output: &mut AudioOutput) -> SynthResult<()> {
        let mut output_module = self.output_module;
        let sample_rate = self.sample_rate;
        let mut sample_clock = self.master_sample_clock.unwrap();

        // Create a callback to pass to CPAL to output the audio. This'll get passed to a different thread that
        // will actually play the audio.
        let output_callback = move |sample_buffer: &mut [T], callback_info: &cpal::OutputCallbackInfo| {
            let buffer_length = sample_buffer.len();
            let mut f32_buffer = Vec::with_capacity(buffer_length);
            for _ in 0..buffer_length {
                f32_buffer.push(0_f32);
            }

            let timestamp = OutputTimestamp::new(callback_info.timestamp());
            let clock_values = sample_clock.get_range(buffer_length);
            let output_info = OutputInfo::new(sample_rate, clock_values, timestamp);

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
            Err(SynthError::new(&msg))
        } else {
            Ok(())
        }
    }
}