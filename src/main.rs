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

enum TestOutputType {
    Noise,
    Sine
}

fn test_output(test_output_type: TestOutputType) -> SynthResult<()> {
    let output_result = output::Output::new(output::OutputDeviceType::Cpal);
    let mut output = match output_result {
        Ok(output) => output,
        Err(err) => {
            let msg = format!("Failed to create output: {}", err);
            return Err(SynthError::new(&msg));
        }
    };

    let cpal_output = match output.get_cpal_mut() {
        Some(cpal_output) => cpal_output,
        None => {
            return Err(SynthError::new("Failed to get CPAL output"));
        }
    };

    match test_output_type {
        TestOutputType::Noise => {
            let sample_format = cpal_output.get_sample_format();
            let sample_output_result = match sample_format {
                cpal::SampleFormat::F32 => cpal_output.set_sample_output(noise::cpal_output_noise::<f32>),
                cpal::SampleFormat::I16 => cpal_output.set_sample_output(noise::cpal_output_noise::<i16>),
                cpal::SampleFormat::U16 => cpal_output.set_sample_output(noise::cpal_output_noise::<u16>)
            };
            if let Err(err) = sample_output_result {
                let msg = format!("Failed to set noise output stream: {}", err);
                return Err(SynthError::new(&msg));
            }
        }

        TestOutputType::Sine => {
            let sample_format = cpal_output.get_sample_format();
            let sample_output_result = match sample_format {
                cpal::SampleFormat::F32 => cpal_output.set_sample_output(oscillator::cpal_output_sine::<f32>),
                cpal::SampleFormat::I16 => cpal_output.set_sample_output(oscillator::cpal_output_sine::<i16>),
                cpal::SampleFormat::U16 => cpal_output.set_sample_output(oscillator::cpal_output_sine::<u16>)
            };
            if let Err(err) = sample_output_result {
                let msg = format!("Failed to set noise output stream: {}", err);
                return Err(SynthError::new(&msg));
            }
        }
    }

    if let Err(err) = cpal_output.play() {
        let msg = format!("Failed to play output stream: {}", err);
        return Err(SynthError::new(&msg));
    }

    // Print cpal debug info
    let cpal_info = cpal_output.get_info();
    println!("Cpal info: {:?}", cpal_info);

    std::thread::sleep(std::time::Duration::from_secs(1));

    Ok(())
}

fn main() -> SynthResult<()> {
    test_output(TestOutputType::Sine)
}
