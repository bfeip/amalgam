extern crate cpal;

use cpal::traits::HostTrait;
use cpal::traits::DeviceTrait;
use cpal::traits::StreamTrait;

use crate::SynthError;
use crate::SynthResult;

/// Structure representing a stream to Cpal. Houses all the info required to output audio
pub struct AudioInterface {
    host: cpal::Host,
    device: cpal::Device,
    current_config: cpal::SupportedStreamConfig,
    stream: cpal::Stream,
    playing: bool
}

/// Structure containing a bunch of info about a `AudioInterface`. Mostly for debugging
#[derive(Debug)]
pub struct CpalInfo {
    pub device_name: String,
    pub sample_rate: u32,
    pub sample_format: cpal::SampleFormat,
    pub channels: u16,
    pub buffer_size: cpal::SupportedBufferSize,
    pub playing: bool
}

impl AudioInterface {
    /// Creates a new `AudioInterface` with default settings. Device and config info are provided by the system.
    /// The default stream callback does nothing so playing this in the default state will do nothing. A stream callback
    /// has to be set before we can output anything interesting.
    pub fn new() -> SynthResult<Self> {
        // Setup the host and device
        let host = cpal::default_host();
        let device = match host.default_output_device() {
            Some(device) => device,
            None => return Err(SynthError::new("No output devices detected"))
        };

        // Get list of supported output configs
        let mut supported_config_range = match device.supported_output_configs() {
            Ok(supported_config_range) => supported_config_range,
            Err(err) => {
                let device_name = device.name();
                if device_name.is_err() {
                    let msg = format!("Not only could we not query the configs for this device. We also couldn't get the device name: {}", err); 
                    return Err(SynthError::new(&msg));
                }
                let device_name = device_name.unwrap(); // shadow
                let msg = format!("Could not query configs for device {}: {}", device_name, err);
                return Err(SynthError::new(&msg));
            }
        };

        // Pick a supported config
        let supported_config = match supported_config_range.next() {
            Some(supported_config) => supported_config,
            None => {
                let device_name = device.name();
                if device_name.is_err() {
                    let msg = "Not only could we not get a supported config. We also couldn't get the device name".to_string(); 
                    return Err(SynthError::new(&msg));
                }
                let device_name = device_name.unwrap(); // shadow
                let msg = format!("No supported configuration for {}", device_name);
                return Err(SynthError::new(&msg));
            }
        };

        let min_sample_rate = supported_config.min_sample_rate();
        let max_sample_rate = supported_config.max_sample_rate();

        // Find a good sample rate. There are a few good rates we'll check for. If we can't get those
        // we'll just go for the max.
        let desired_sample_rates = [
            cpal::SampleRate(48_000),
            cpal::SampleRate(44_100),
        ];
        let mut sample_rate = max_sample_rate;
        for desired_sample_rate in desired_sample_rates.iter().cloned() {
            if desired_sample_rate > min_sample_rate && desired_sample_rate < max_sample_rate {
                sample_rate = desired_sample_rate;
                break;
            }
        };
        let current_config = supported_config.with_sample_rate(sample_rate);

        // Start setting up the output stream
        let sample_format = current_config.sample_format();
        let stream_result = match sample_format {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &current_config.config(), null_stream_callback::<f32>, print_error_callback
            ),
            cpal::SampleFormat::I16 => device.build_output_stream(
                &current_config.config(), null_stream_callback::<i16>, print_error_callback
            ),
            cpal::SampleFormat::U16 => device.build_output_stream(
                &current_config.config(), null_stream_callback::<u16>, print_error_callback
            ),
        };

        let stream = match stream_result {
            Ok(stream) => stream,
            Err(err) => {
                let msg = format!("Failed to create a cpal output stream: {}", err);
                return Err(SynthError::new(&msg));
            }
        };

        let playing = false;
        Ok(AudioInterface {
            host, device, current_config, stream, playing
        })
    }

    pub fn get_channel_count(&self) -> cpal::ChannelCount {
        self.current_config.channels()
    }

    /// Gets the format the samples are in i.e. u16, i16, or f32. See `cpal::SampleFormat` for more details.
    pub fn get_sample_format(&self) -> cpal::SampleFormat {
        self.current_config.sample_format()
    }

    /// Gets the sample rate of the current config
    pub fn get_sample_rate(&self) -> cpal::SampleRate {
        self.current_config.sample_rate()
    }

    /// Sets the callback that will be called upon to fill the samples provided by Cpal. The callback you provide should
    /// fill the samples in `data` with the audio you want to output. 
    pub fn set_stream_callback<
        T: cpal::Sample,
        D: FnMut(&mut [T], &cpal::OutputCallbackInfo) + Send + 'static
    > (&mut self, sample_output: D) -> SynthResult<()> {
        let stream_result = self.device.build_output_stream(&self.current_config.config(), sample_output, print_error_callback);

        let stream = match stream_result {
            Ok(stream) => stream,
            Err(err) => {
                let msg = format!("Failed to create a cpal output stream: {}", err);
                return Err(SynthError::new(&msg));
            }
        };

        self.stream = stream;

        Ok(())
    }

    /// Starts playing the audio stream
    pub fn play(&mut self) -> SynthResult<()> {
        if let Err(err) = self.stream.play() {
            let msg = format!("Failed to begin stream playback: {}", err);
            return Err(SynthError::new(&msg));
        }
        self.playing = true;
        Ok(())
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Gets a bunch of info about this struct and puts it into an easily printable `CpalInfo`
    pub fn get_info(&self) -> CpalInfo {
        let device_name = match self.device.name() {
            Ok(device_name) => device_name,
            Err(_err) => "Failed to get device name".to_string()
        };
        let sample_rate = self.get_sample_rate().0;
        let sample_format = self.get_sample_format();
        let channels = self.current_config.channels();
        let buffer_size = self.current_config.buffer_size().clone();
        let playing = self.playing;

        CpalInfo {
            device_name,
            sample_rate,
            sample_format,
            channels,
            buffer_size,
            playing
        }
    }
}

/// The default stream callback. Does nothing but write 0s to the audio stream
fn null_stream_callback<T: cpal::Sample>(data: &mut [T], _: &cpal::OutputCallbackInfo) {
    for sample in data.iter_mut() {
        *sample = cpal::Sample::from(&0.0);
    }
}

/// A stream error callback that does nothing
fn null_error_callback(_err: cpal::StreamError) {

}

/// A stream error callback that simply prints the error it gets passed
fn print_error_callback(err: cpal::StreamError) {
    println!("CPAL ERROR: {}", err)
}