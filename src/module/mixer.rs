use crate::prelude::*;
use super::traits::SignalOutputModule;
use super::error::*;
use super::empty::Empty;

struct MixerInput {
    input: Box<dyn SignalOutputModule>,
    level: f32
}

impl MixerInput {
    fn new() -> Self {
        let input = Box::new(Empty::new());
        let level = 1_f32;
        Self { input, level }
    }

    fn with_input(input: Box<dyn SignalOutputModule>) -> Self {
        let level = 1_f32;
        Self { input, level }
    }
}

#[derive(Copy, Clone)]
enum MixerCompressMode {
    None,
    Compress,
    Limit
}

struct Mixer {
    inputs: Vec<MixerInput>,
    compression_mode: MixerCompressMode
}

impl Mixer {
    fn new() -> Self {
        let inputs = Vec::new();
        let compression_mode = MixerCompressMode::None;
        Self { inputs, compression_mode }
    }

    fn with_inputs(n_inputs: usize) -> Self {
        let mut inputs = Vec::with_capacity(n_inputs);
        for _ in 0..n_inputs {
            inputs.push(MixerInput::new());
        }
        let compression_mode = MixerCompressMode::None;
        Self { inputs, compression_mode }
    }

    fn add_input(&mut self, input: MixerInput) {
        self.inputs.push(input);
    }

    fn remove_input(&mut self, input_index: usize) -> ModuleResult<()> {
        if input_index > self.inputs.len() {
            let msg = "Tried to remove element from mixer that was out of bounds";
            return Err(ModuleError::new(msg));
        }
        self.inputs.remove(input_index);
        Ok(())
    }
}

impl SignalOutputModule for Mixer {
    fn fill_output_buffer(&mut self, data: &mut [f32]) {
        let data_len = data.len();
        let input_len = self.inputs.len();

        for datum in data.iter_mut() {
            *datum = 0.0;
        }

        // Merge all inputs into `data`
        let mut data_buffer = Vec::with_capacity(data_len);
        data_buffer.resize(data_len, 0.0);
        for i in 0..input_len {
            let input = &mut self.inputs[i];
            input.input.fill_output_buffer(&mut data_buffer);

            // Apply the level if we need to
            if !float_eq(input.level, 1.0, 0.000001) {
                for datum in data_buffer.iter_mut() {
                    *datum *= input.level;
                }
            }
            
            for i in 0..data_len {
                *&mut data[i] += data_buffer[i];
            }
        }

        // Apply compression if needed
        match self.compression_mode {
            MixerCompressMode::None => return,
            MixerCompressMode::Compress => {
                // TODO: This might be the poor man's compression. Should research into doing it proper
                // Find largest element of the buffer
                let mut largest_element = 0.0;
                for datum in data.iter() {
                    let datum_abs = f32::abs(*datum);
                    if datum_abs > largest_element {
                        largest_element = datum_abs;
                    }
                }

                if largest_element < 1.0 {
                    // If we're always below the limit then don't try to reduce
                    return;
                }

                // Reduce all elements by a factor that makes the peaks 1.0 or -1.0
                let reduction_factor = largest_element;
                for datum in data.iter_mut() {
                    *datum /= reduction_factor;
                }
            }
            MixerCompressMode::Limit => {
                for datum in data.iter_mut() {
                    if *datum > 1.0 {
                        *datum = 1.0;
                    } 
                    else if *datum < -1.0 {
                        *datum = -1.0;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::oscillator;

    fn get_square_and_25_pulse_mixer_inputs(sample_rate: usize) -> (MixerInput, MixerInput) {
        let mut osc1_state = oscillator::OscillatorState::new(sample_rate as f32);
        osc1_state.frequency = 1.0;
        osc1_state.waveform = oscillator::Waveform::Pulse;
        osc1_state.pulse_width = 0.5;
        let mut osc2_state = osc1_state.clone();
        osc2_state.pulse_width = 0.25;
        let osc1 = oscillator::Oscillator::from_state(&osc1_state);
        let osc2 = oscillator::Oscillator::from_state(&osc2_state);
        let mixer_input_1 = MixerInput::with_input(Box::new(osc1));
        let mixer_input_2 = MixerInput::with_input(Box::new(osc2));
        (mixer_input_1, mixer_input_2)
    }

    #[test]
    fn test_no_compression_mixing() {
        const SAMPLE_RATE: usize = 10_usize;
        const EXPECTED_DATA: [f32; SAMPLE_RATE] = [-2.0, -2.0, 0.0, 0.0, 0.0, 2.0, 2.0, 2.0, 2.0, -2.0];
        let mut mixer = Mixer::new();
        mixer.compression_mode = MixerCompressMode::None;

        let (mixer_input_1, mixer_input_2) = get_square_and_25_pulse_mixer_inputs(SAMPLE_RATE);

        mixer.add_input(mixer_input_1);
        mixer.add_input(mixer_input_2);

        let mut output_buffer = Vec::with_capacity(SAMPLE_RATE);
        output_buffer.resize(SAMPLE_RATE, 0.0);
        mixer.fill_output_buffer(&mut output_buffer);

        for i in 0..SAMPLE_RATE {
            assert!(
                float_eq(EXPECTED_DATA[i], output_buffer[i], 0.000001),
                "Output does not match expected.\n\tExpected: {:?}\n\tGot: {:?}", EXPECTED_DATA, output_buffer
            );
        }
    }

    #[test]
    fn test_level_mixing() {
        const SAMPLE_RATE: usize = 10_usize;
        const EXPECTED_DATA: [f32; SAMPLE_RATE] = [-1.5, -1.5, 0.5, 0.5, 0.5, 1.5, 1.5, 1.5, 1.5, -1.5];
        let mut mixer = Mixer::new();
        mixer.compression_mode = MixerCompressMode::None;

        let (mut mixer_input_1, mixer_input_2) = get_square_and_25_pulse_mixer_inputs(SAMPLE_RATE);
        mixer_input_1.level = 0.5;

        mixer.add_input(mixer_input_1);
        mixer.add_input(mixer_input_2);

        let mut output_buffer = Vec::with_capacity(SAMPLE_RATE);
        output_buffer.resize(SAMPLE_RATE, 0.0);
        mixer.fill_output_buffer(&mut output_buffer);

        for i in 0..SAMPLE_RATE {
            assert!(
                float_eq(EXPECTED_DATA[i], output_buffer[i], 0.000001),
                "Output does not match expected.\n\tExpected: {:?}\n\tGot: {:?}", EXPECTED_DATA, output_buffer
            );
        }
    }

    #[test]
    fn test_compression_mixing() {
        const SAMPLE_RATE: usize = 10_usize;
        const EXPECTED_DATA: [f32; SAMPLE_RATE] = [-1.0, -1.0, 1.0/3.0, 1.0/3.0, 1.0/3.0, 1.0, 1.0, 1.0, 1.0, -1.0];
        let mut mixer = Mixer::new();
        mixer.compression_mode = MixerCompressMode::Compress;

        let (mut mixer_input_1, mixer_input_2) = get_square_and_25_pulse_mixer_inputs(SAMPLE_RATE);
        mixer_input_1.level = 0.5;

        mixer.add_input(mixer_input_1);
        mixer.add_input(mixer_input_2);

        let mut output_buffer = Vec::with_capacity(SAMPLE_RATE);
        output_buffer.resize(SAMPLE_RATE, 0.0);
        mixer.fill_output_buffer(&mut output_buffer);

        for i in 0..SAMPLE_RATE {
            assert!(
                float_eq(EXPECTED_DATA[i], output_buffer[i], 0.000001),
                "Output does not match expected.\n\tExpected: {:?}\n\tGot: {:?}", EXPECTED_DATA, output_buffer
            );
        }
    }

    #[test]
    fn test_limit_mixing() {
        const SAMPLE_RATE: usize = 10_usize;
        const EXPECTED_DATA: [f32; SAMPLE_RATE] = [-1.0, -1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, -1.0];
        let mut mixer = Mixer::new();
        mixer.compression_mode = MixerCompressMode::Limit;

        let (mixer_input_1, mixer_input_2) = get_square_and_25_pulse_mixer_inputs(SAMPLE_RATE);

        mixer.add_input(mixer_input_1);
        mixer.add_input(mixer_input_2);

        let mut output_buffer = Vec::with_capacity(SAMPLE_RATE);
        output_buffer.resize(SAMPLE_RATE, 0.0);
        mixer.fill_output_buffer(&mut output_buffer);

        for i in 0..SAMPLE_RATE {
            assert!(
                float_eq(EXPECTED_DATA[i], output_buffer[i], 0.000001),
                "Output does not match expected.\n\tExpected: {:?}\n\tGot: {:?}", EXPECTED_DATA, output_buffer
            );
        }
    }
}