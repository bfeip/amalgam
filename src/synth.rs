use cpal::SupportedBufferSize;
use crate::module::common::{SynthModule, OutputInfo};

use super::error::{SynthResult, SynthError};
use super::module::Output;
use super::output::AudioInterface;
use super::clock;
use super::SignalLogger;

use std::sync::{Arc, Mutex};

pub struct Synth {
    audio_interface: AudioInterface,
    output_module: Output,
    sample_rate: usize,
    master_sample_clock: clock::SampleClock,
    signal_logger: SignalLogger,
    audio_queue: Arc<Mutex<Vec<f32>>>
}

impl Synth {
    pub fn new() -> SynthResult<Self> {
        let audio_interface = match AudioInterface::new() {
            Ok(audio_interface) => audio_interface,
            Err(error) => {
                let msg = format!("Failed to create audio interface: {}", error);
                return Err(SynthError::new(&msg));
            }
        };

        let sample_rate = audio_interface.get_sample_rate().0 as usize;

        let output_module = Output::new();
        let master_sample_clock = clock::SampleClock::new(sample_rate);

        #[cfg(feature = "signal_logging")]
        let signal_logger = SignalLogger::new("final_signal.txt");
        #[cfg(not(feature = "signal_logging"))]
        let signal_logger = SignalLogger::new_sink();

        let audio_queue = Arc::new(Mutex::new(Vec::new()));

        let synth = Synth {
            audio_interface,
            output_module,
            sample_rate,
            master_sample_clock,
            signal_logger,
            audio_queue
        };
        Ok(synth)
    }

    pub fn get_output_module(&self) -> &Output {
        &self.output_module
    }

    pub fn get_output_module_mut(&mut self) -> &mut Output {
        &mut self.output_module
    }

    fn init_cpal_callback<T: cpal::Sample>(&mut self) -> SynthResult<()> {
        let audio_queue = self.audio_queue.clone();
        let callback = move |audio: &mut [T], _callback_info: &cpal::OutputCallbackInfo| {
            if let Ok(mut audio_queue) = audio_queue.lock() {
                if audio_queue.len() < audio.len() {
                    // We do not have enough audio to play. I think we just play nothing
                    audio.fill(T::from(&0_i16));
                    return;
                }

                // Fill the audio buffer with the pre-processed audio
                let float_audio = audio_queue.drain(0..audio.len());
                debug_assert!(float_audio.len() == audio.len());
                for (datum, float_datum) in audio.iter_mut().zip(float_audio) {
                    *datum = T::from(&float_datum);
                }
            }
            else {
                // We failed to lock the audio_queue which means that something has gone horribly wrong
                audio.fill(T::from(&0_i16));
                return;
            }
        };

        self.audio_interface.set_stream_callback(callback)?;
        Ok(())
    }

    pub fn play(&mut self) -> SynthResult<()> {
        match self.audio_interface.get_sample_format() {
            cpal::SampleFormat::I16 => self.init_cpal_callback::<i16>()?,
            cpal::SampleFormat::U16 => self.init_cpal_callback::<u16>()?,
            cpal::SampleFormat::F32 => self.init_cpal_callback::<f32>()?,
        };
        self.audio_interface.play()
    }

    pub fn gen_samples(&mut self) -> SynthResult<()> {
        if !self.audio_interface.is_playing() {
            let msg = "Cannot generate samples while not playing";
            return Err(SynthError::new(msg));
        }

        // A quick and dirty stop to generate no more than one second of audio
        if let Ok(audio_queue) = self.audio_queue.lock() {
            if audio_queue.len() > self.sample_rate {
                return Ok(());
            }
        }

        let cpal_info = self.audio_interface.get_info();
        let channels = cpal_info.channels as usize;
        let n_mono_samples = match cpal_info.buffer_size {
            SupportedBufferSize::Range { min: _, max } => {
                max as usize
            },
            SupportedBufferSize::Unknown => {
                // uhhhhh
                10_000
            }
        };
        let mut multi_channel_audio = vec![0_f32; n_mono_samples * channels];

        let output_info = OutputInfo::new(
            cpal_info.sample_rate as usize,
            cpal_info.channels,
            self.master_sample_clock.get_range(n_mono_samples),
            std::time::Instant::now() // wrong
        );
        self.output_module.fill_output_buffer(&mut multi_channel_audio, &output_info);

        if let Ok(mut audio_queue) = self.audio_queue.lock() {
            audio_queue.append(&mut multi_channel_audio);
        }

        Ok(())
    }
}