mod error;
mod cpal_audio_output;

use error::{AudioOutputResult, AudioOutputError};
use cpal_audio_output::{CpalAudioOutput, CpalInfo};

/// Represents a kind of audio output device. Currently Cpal is the only supported type
pub enum OutputDeviceType {
    Cpal
}

/// A structure representing what would be an output module on a modular synth. Currently it just wraps the `CpalAudioOutput`.
/// Somewhat useless. I'm not sure if it'll ne sticking around
pub struct AudioOutput {
    cpal_out: Option<CpalAudioOutput>
}

impl AudioOutput {
    /// Creates a new `Output` and initializes an audio output device based upon the provided
    /// `output_device_type`
    pub fn new(output_device_type: OutputDeviceType) -> AudioOutputResult<Self> {
        match output_device_type {
            OutputDeviceType::Cpal => {
                let cpal_out = match CpalAudioOutput::new() {
                    Ok(cpal_out) => Some(cpal_out),
                    Err(err) => {
                        let msg = format!("Failed to make Cpal output: {}", err);
                        return Err(AudioOutputError::new(&msg));
                    }
                };
                Ok(AudioOutput{ cpal_out })
            }
        }
    }

    /// Sets the cpal stream callback used to fill samples that are to be written to the audio stream
    /// See `CpalAudioOutput` for more details
    pub fn set_output_callback<
        T: cpal::Sample,
        D: FnMut(&mut [T], &cpal::OutputCallbackInfo) + Send + 'static
    >(&mut self, sample_output_callback: D) -> AudioOutputResult<()> {
        if let Some(cpal_out) = &mut self.cpal_out {
            if let Err(err) = cpal_out.set_stream_callback(sample_output_callback) {
                let msg = format!("Failed to set output callback: {}", err);
                return Err(AudioOutputError::new(&msg))
            }
            return Ok(())
        }
        else {
            return Err(AudioOutputError::new("No Cpal output"))
        }
    }

    pub fn get_channel_count(&self) -> Option<cpal::ChannelCount> {
        match &self.cpal_out {
            Some(cpal_out) => Some(cpal_out.get_channel_count()),
            None => None
        }
    }

    pub fn get_sample_format(&self) -> Option<cpal::SampleFormat> {
        match &self.cpal_out {
            Some(cpal_out) => Some(cpal_out.get_sample_format()),
            None => None
        }
    }

    pub fn get_sample_rate(&self) -> Option<cpal::SampleRate> {
        match &self.cpal_out {
            Some(cpal_out) => Some(cpal_out.get_sample_rate()),
            None => None
        }
    }

    pub fn get_cpal_info(&self) -> Option<CpalInfo> {
        match &self.cpal_out {
            Some(cpal_out) => Some(cpal_out.get_info()),
            None => None
        }
    }

    /// Gets a reference to the Cpal output if there is one
    pub fn get_cpal(&self) -> &Option<CpalAudioOutput> {
        &self.cpal_out
    }

    /// Gets a mutable reference to the Cpal output if there is one
    pub fn get_cpal_mut(&mut self) -> &mut Option<CpalAudioOutput> {
        &mut self.cpal_out
    }

    pub fn play(&mut self) -> AudioOutputResult<()> {
        match self.get_cpal_mut() {
            Some(cpal_out) => cpal_out.play(),
            None => Err(AudioOutputError::new("Failed to play stream because there's no cpal output"))
        }
    }
}