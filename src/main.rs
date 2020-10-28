#![allow(dead_code)]

extern crate cpal;

mod error;
mod oscillator;
mod prelude;
mod note;
mod clock;
mod noise;
mod output;

use crate::error::{SynthError, SynthResult};

/// Represents a type of audio test to be preformed
enum TestOutputType {
    /// Test should output white noise
    Noise,
    /// Test should output a sine wave
    Sine
}

/// Outputs a second of audio who's source depends on the `test_output_type` you provided
fn test_output(test_output_type: TestOutputType) -> SynthResult<()> {
    // create an output with cpal as the backend. I'm not planning on adding any other backends right now
    // since cpal is nice and low level so the output struct is a useless abstraction that should be removed
    // in the future
    let output_result = output::Output::new(output::OutputDeviceType::Cpal);
    let mut output = match output_result {
        Ok(output) => output,
        Err(err) => {
            let msg = format!("Failed to create output: {}", err);
            return Err(SynthError::new(&msg));
        }
    };

    // Get the cpal output. We'll just interact with it directly instead of writing methods in the  (mostly useless)
    // `Output` struct
    let cpal_output = match output.get_cpal_mut() {
        Some(cpal_output) => cpal_output,
        None => {
            return Err(SynthError::new("Failed to get CPAL output"));
        }
    };

    // TODO: This is just a test function so it's not a big deal right now but this is very verbose
    match test_output_type {
        TestOutputType::Noise => {
            let sample_format = cpal_output.get_sample_format();
            let sample_output_result = match sample_format {
                cpal::SampleFormat::F32 => cpal_output.set_stream_callback(noise::cpal_output_noise::<f32>),
                cpal::SampleFormat::I16 => cpal_output.set_stream_callback(noise::cpal_output_noise::<i16>),
                cpal::SampleFormat::U16 => cpal_output.set_stream_callback(noise::cpal_output_noise::<u16>)
            };
            if let Err(err) = sample_output_result {
                let msg = format!("Failed to set noise output stream: {}", err);
                return Err(SynthError::new(&msg));
            }
        }

        TestOutputType::Sine => {
            let sample_format = cpal_output.get_sample_format();
            let sample_output_result = match sample_format {
                cpal::SampleFormat::F32 => cpal_output.set_stream_callback(oscillator::cpal_output_sine::<f32>),
                cpal::SampleFormat::I16 => cpal_output.set_stream_callback(oscillator::cpal_output_sine::<i16>),
                cpal::SampleFormat::U16 => cpal_output.set_stream_callback(oscillator::cpal_output_sine::<u16>)
            };
            if let Err(err) = sample_output_result {
                let msg = format!("Failed to set noise output stream: {}", err);
                return Err(SynthError::new(&msg));
            }
        }
    }

    // Begin playing the audio
    if let Err(err) = cpal_output.play() {
        let msg = format!("Failed to play output stream: {}", err);
        return Err(SynthError::new(&msg));
    }

    // Print cpal debug info
    let cpal_info = cpal_output.get_info();
    println!("Cpal info: {:?}", cpal_info);

    // The audio is being played on a separate thread owned by cpal if I understand correctly
    // so we need to sleep here to give it enough time to play a bit
    std::thread::sleep(std::time::Duration::from_secs(1));

    Ok(())
}

fn main() -> SynthResult<()> {
    test_output(TestOutputType::Sine)
}
