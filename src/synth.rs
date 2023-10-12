use crate::module::ModuleManager;

use super::error::{SynthResult, SynthError};
use super::module::Output;
use super::module::common::{SignalOutputModule, OutputInfo, OutputTimestamp};
use super::output::AudioInterface;
use super::clock;
use super::SignalLogger;

#[cfg(feature = "audio_printing")]
use std::time;

use std::sync::{Arc, Mutex};

pub struct Synth {
    output_module: Output,
    sample_rate: usize,
    master_sample_clock: clock::SampleClock,
    signal_logger: SignalLogger,
    module_manager: ModuleManager
}

impl Synth {
    pub fn new() -> SynthResult<Self> {
        let output_module = Output::new();
        let master_sample_clock = clock::SampleClock::new(0);
        let sample_rate = 0;

        #[cfg(feature = "signal_logging")]
        let signal_logger = SignalLogger::new("final_signal.txt");
        #[cfg(not(feature = "signal_logging"))]
        let signal_logger = SignalLogger::new_sink();
        let module_manager = ModuleManager::new();

        let synth = Synth { output_module, sample_rate, master_sample_clock, signal_logger, module_manager };
        Ok(synth)
    }

    pub fn get_output_module(&self) -> &Output {
        &self.output_module
    }

    pub fn get_output_module_mut(&mut self) -> &mut Output {
        &mut self.output_module
    }

    pub fn play(synth: Arc<Mutex<Self>>, audio_interface: &mut AudioInterface) -> SynthResult<()> {
        let sample_type = audio_interface.get_sample_format();

        // Get and store the sample rate
        let new_sample_rate = audio_interface.get_sample_rate().0 as usize;

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
            if sample_rate_has_changed {
                let clock = clock::SampleClock::new(new_sample_rate);
                locked_synth.master_sample_clock = clock;
            }
        }

        match sample_type {
            cpal::SampleFormat::F32 => Self::play_with_cpal::<f32>(synth, audio_interface),
            cpal::SampleFormat::I16 => Self::play_with_cpal::<i16>(synth, audio_interface),
            cpal::SampleFormat::U16 => Self::play_with_cpal::<u16>(synth, audio_interface)
        }
    }

    fn play_with_cpal<T: cpal::Sample>(synth: Arc<Mutex<Self>>, audio_interface: &mut AudioInterface) -> SynthResult<()> {
        let channel_count = audio_interface.get_channel_count();

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

            let buffer_length = sample_buffer.len();
            let mut f32_buffer = Vec::with_capacity(buffer_length);
            for _ in 0..buffer_length {
                f32_buffer.push(0_f32);
            }

            let timestamp = OutputTimestamp::new(callback_info.timestamp());

            let clock_values_len = buffer_length / channel_count as usize;
            let mut sample_clock = locked_synth.master_sample_clock;
            let sample_range = sample_clock.get_range(clock_values_len);
            locked_synth.master_sample_clock = sample_clock;
            let output_info = OutputInfo::new(locked_synth.sample_rate, channel_count, sample_range, timestamp);

            #[cfg(feature = "audio_printing")]
            let computation_started = time::Instant::now();
            locked_synth.output_module.fill_output_buffer(&mut f32_buffer, &output_info, &mut locked_synth.module_manager);
            for i in 0..buffer_length {
                sample_buffer[i] = T::from(&f32_buffer[i]);
            }
            #[cfg(feature = "audio_printing")]
            let computation_ended = time::Instant::now();

            if let Err(err) = locked_synth.signal_logger.log("final".to_owned(), &f32_buffer) {
                panic!("Failed to write signal log: {}", err);
            }

            #[cfg(feature = "audio_printing")]
            {
                let computation_duration = computation_ended - computation_started;
                let audio_duration = callback_info.timestamp().playback.duration_since(
                    &callback_info.timestamp().callback
                ).unwrap();
                
                let mut max_sample_diff = 0_f32;
                for sample_pair in f32_buffer.windows(2) {
                    let sample_diff = (sample_pair[0] - sample_pair[1]).abs();
                    max_sample_diff = max_sample_diff.max(sample_diff);
                }

                println!(
                    concat!(
                        "{{",
                        "\tComputation duration: {:#?}\n",
                        "\tAudio duration: {:#?}\n",
                        "\tMax sample diff: {}\n",
                        "}}"
                    ),
                    computation_duration, audio_duration, max_sample_diff
                )
            }
        };

        if let Err(err) = audio_interface.set_stream_callback(output_callback) {
            let msg = format!("Failed to play from Synth because we couldn't set the output callback: {}", err);
            return Err(SynthError::new(&msg));
        }

        if let Err(err) = audio_interface.play() {
            let msg = format!("Failed to play audio stream: {}", err);
            return Err(SynthError::new(&msg))
        }

        Ok(())
    }
}