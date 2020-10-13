mod error;
mod cpal_audio_output;

use error::{AudioOutputResult, AudioOutputError};
use cpal_audio_output::CpalAudioOutput;

pub enum OutputDeviceType {
    Cpal
}

pub struct Output {
    cpal_out: Option<CpalAudioOutput>
}

impl Output {
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
                Ok(Output{ cpal_out })
            }
        }
    }

    pub fn set_output_callback<
        T: cpal::Sample,
        D: FnMut(&mut [T], &cpal::OutputCallbackInfo) + Send + 'static
    >(&mut self, sample_output_callback: D) -> AudioOutputResult<()> {
        if let Some(cpal_out) = &mut self.cpal_out {
            if let Err(err) = cpal_out.set_sample_output(sample_output_callback) {
                let msg = format!("Failed to set output callback: {}", err);
                return Err(AudioOutputError::new(&msg))
            }
            return Ok(())
        }
        else {
            return Err(AudioOutputError::new("No Cpal output"))
        }
    }

    pub fn get_cpal(&self) -> &Option<CpalAudioOutput> {
        &self.cpal_out
    }

    pub fn get_cpal_mut(&mut self) -> &mut Option<CpalAudioOutput> {
        &mut self.cpal_out
    }
}