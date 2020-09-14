extern crate cpal;

use cpal::Data;
use cpal::traits::HostTrait;
use cpal::traits::DeviceTrait;
use cpal::traits::StreamTrait;

use super::error::{AudioOutputError, AudioOutputResult};

pub struct CpalAudioOutput {
    host: cpal::Host,
    device: cpal::Device,
    current_config: cpal::SupportedStreamConfig,
    stream: cpal::Stream,
}

impl CpalAudioOutput {
    pub fn new() -> AudioOutputResult<Self> {
        // Setup the host and device
        let host = cpal::default_host();
        let device = match host.default_output_device() {
            Some(device) => device,
            None => return Err(AudioOutputError::new("No output devices detected"))
        };

        // Get list of supported output configs
        let mut supported_config_range = match device.supported_output_configs() {
            Ok(supported_config_range) => supported_config_range,
            Err(err) => {
                let device_name = device.name();
                if device_name.is_err() {
                    let msg = format!("Not only could we not query the config for this device. We also couldn't get the device name: {}", err); 
                    return Err(AudioOutputError::new(&msg));
                }
                let device_name = device_name.unwrap(); // shadow
                let msg = format!("Could not query configs for device {}: {}", device_name, err);
                return Err(AudioOutputError::new(&msg));
            }
        };

        // Pick a supported config
        let supported_config = match supported_config_range.next() {
            Some(supported_config) => supported_config,
            None => {
                let device_name = device.name();
                if device_name.is_err() {
                    let msg = format!("Not only could we not get a supported config. We also couldn't get the device name"); 
                    return Err(AudioOutputError::new(&msg));
                }
                let device_name = device_name.unwrap(); // shadow
                let msg = format!("No supported configuration for {}", device_name);
                return Err(AudioOutputError::new(&msg));
            }
        };

        let current_config = supported_config.with_max_sample_rate();

        // Start setting up the output stream
        let sample_format = current_config.sample_format();
        let stream_result = match sample_format {
            cpal::SampleFormat::F32 => device.build_output_stream(&current_config.config(), null_stream_callback::<f32>, null_error_callback),
            cpal::SampleFormat::I16 => device.build_output_stream(&current_config.config(), null_stream_callback::<i16>, null_error_callback),
            cpal::SampleFormat::U16 => device.build_output_stream(&current_config.config(), null_stream_callback::<u16>, null_error_callback),
        };

        let stream = match stream_result {
            Ok(stream) => stream,
            Err(err) => {
                let msg = format!("Failed to create a cpal output stream: {}", err);
                return Err(AudioOutputError::new(&msg));
            }
        };

        Ok(CpalAudioOutput{ host, device, current_config, stream })
    }

    pub fn get_sample_format(&self) -> cpal::SampleFormat {
        self.current_config.sample_format()
    }

    pub fn get_sample_rate(&self) {
        self.current_config.sample_rate();
    }
}

fn null_stream_callback<T: cpal::Sample>(data: &mut [T], _: &cpal::OutputCallbackInfo) {
    for sample in data.iter_mut() {
        *sample = cpal::Sample::from(&0.0);
    }
}

fn null_error_callback(_err: cpal::StreamError) {

}