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

    pub fn get_cpal(&self) -> &Option<CpalAudioOutput> {
        &self.cpal_out
    }

    pub fn get_cpal_mut(&mut self) -> &mut Option<CpalAudioOutput> {
        &mut self.cpal_out
    }
}