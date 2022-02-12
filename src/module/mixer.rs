use crate::prelude::*;
use super::common::{SignalOutputModule, OutputInfo, CompressionMode, compress_audio, Connectable};
use super::error::*;
use super::empty::Empty;

pub struct MixerInput {
    input: Connectable<dyn SignalOutputModule>,
    level: f32
}

impl MixerInput {
    pub fn new() -> Self {
        let input = Empty::new().into();
        let level = 1_f32;
        Self { input, level }
    }

    pub fn with_input(input: Connectable<dyn SignalOutputModule>) -> Self {
        let level = 1_f32;
        Self { input, level }
    }

    pub fn set_input(&mut self, input: Connectable<dyn SignalOutputModule>) {
        self.input = input;
    }

    pub fn set_level(&mut self, level: f32) {
        self.level = level;
    }
}

pub struct Mixer {
    inputs: Vec<MixerInput>,
    compression_mode: CompressionMode
}

impl Mixer {
    pub fn new() -> Self {
        let inputs = Vec::new();
        let compression_mode = CompressionMode::None;
        Self { inputs, compression_mode }
    }

    pub fn with_inputs(n_inputs: usize) -> Self {
        let mut inputs = Vec::with_capacity(n_inputs);
        for _ in 0..n_inputs {
            inputs.push(MixerInput::new());
        }
        let compression_mode = CompressionMode::None;
        Self { inputs, compression_mode }
    }

    pub fn add_input(&mut self, input: MixerInput) {
        self.inputs.push(input);
    }

    pub fn remove_input(&mut self, input_index: usize) -> ModuleResult<()> {
        if input_index > self.inputs.len() {
            let msg = "Tried to remove element from mixer that was out of bounds";
            return Err(ModuleError::new(msg));
        }
        self.inputs.remove(input_index);
        Ok(())
    }

    pub fn iter_inputs(&self) -> std::slice::Iter<MixerInput> {
        self.inputs.iter()
    }

    pub fn iter_inputs_mut(&mut self) -> std::slice::IterMut<MixerInput> {
        self.inputs.iter_mut()
    }
}

impl SignalOutputModule for Mixer {
    fn fill_output_buffer(&mut self, data: &mut [f32], output_info: &OutputInfo) {
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
            let mut signal_input_lock = input.input.lock();
            signal_input_lock.fill_output_buffer(&mut data_buffer, output_info);

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
        compress_audio(data, self.compression_mode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::oscillator;
    use crate::clock;

    fn get_square_and_25_pulse_mixer_inputs() -> (MixerInput, MixerInput) {
        let mut osc1 = oscillator::Oscillator::new();
        osc1.set_frequency(1.0);
        osc1.set_waveform(oscillator::Waveform::Pulse);
        osc1.set_pulse_width(0.5);
        let mut osc2 = osc1.clone();
        osc2.set_pulse_width(0.25);
        let mixer_input_1 = MixerInput::with_input(osc1.into());
        let mixer_input_2 = MixerInput::with_input(osc2.into());
        (mixer_input_1, mixer_input_2)
    }

    fn get_clock_values(sample_rate: usize, buffer_size: usize) -> Vec<usize> {
        let mut clock = clock::SampleClock::new(sample_rate);
        clock.get_range(buffer_size)
    }

    #[test]
    fn test_no_compression_mixing() {
        const SAMPLE_RATE: usize = 10_usize;
        const EXPECTED_DATA: [f32; SAMPLE_RATE] = [-2.0, -2.0, 0.0, 0.0, 0.0, 2.0, 2.0, 2.0, 2.0, -2.0];
        let mut mixer = Mixer::new();
        mixer.compression_mode = CompressionMode::None;

        let (mixer_input_1, mixer_input_2) = get_square_and_25_pulse_mixer_inputs();

        mixer.add_input(mixer_input_1);
        mixer.add_input(mixer_input_2);

        let clock_values = get_clock_values(SAMPLE_RATE, EXPECTED_DATA.len());
        let output_info = OutputInfo::new_basic(SAMPLE_RATE, clock_values);

        let mut output_buffer = Vec::with_capacity(SAMPLE_RATE);
        output_buffer.resize(SAMPLE_RATE, 0.0);
        mixer.fill_output_buffer(&mut output_buffer, &output_info);

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
        mixer.compression_mode = CompressionMode::None;

        let (mut mixer_input_1, mixer_input_2) = get_square_and_25_pulse_mixer_inputs();
        mixer_input_1.level = 0.5;

        mixer.add_input(mixer_input_1);
        mixer.add_input(mixer_input_2);
        
        let clock_values = get_clock_values(SAMPLE_RATE, EXPECTED_DATA.len());
        let output_info = OutputInfo::new_basic(SAMPLE_RATE, clock_values);

        let mut output_buffer = Vec::with_capacity(SAMPLE_RATE);
        output_buffer.resize(SAMPLE_RATE, 0.0);
        mixer.fill_output_buffer(&mut output_buffer, &output_info);

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
        mixer.compression_mode = CompressionMode::Compress;

        let (mut mixer_input_1, mixer_input_2) = get_square_and_25_pulse_mixer_inputs();
        mixer_input_1.level = 0.5;

        mixer.add_input(mixer_input_1);
        mixer.add_input(mixer_input_2);

        let clock_values = get_clock_values(SAMPLE_RATE, EXPECTED_DATA.len());
        let output_info = OutputInfo::new_basic(SAMPLE_RATE, clock_values);

        let mut output_buffer = Vec::with_capacity(SAMPLE_RATE);
        output_buffer.resize(SAMPLE_RATE, 0.0);
        mixer.fill_output_buffer(&mut output_buffer, &output_info);

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
        mixer.compression_mode = CompressionMode::Limit;

        let (mixer_input_1, mixer_input_2) = get_square_and_25_pulse_mixer_inputs();

        mixer.add_input(mixer_input_1);
        mixer.add_input(mixer_input_2);

        let clock_values = get_clock_values(SAMPLE_RATE, EXPECTED_DATA.len());
        let output_info = OutputInfo::new_basic(SAMPLE_RATE, clock_values);

        let mut output_buffer = Vec::with_capacity(SAMPLE_RATE);
        output_buffer.resize(SAMPLE_RATE, 0.0);
        mixer.fill_output_buffer(&mut output_buffer, &output_info);

        for i in 0..SAMPLE_RATE {
            assert!(
                float_eq(EXPECTED_DATA[i], output_buffer[i], 0.000001),
                "Output does not match expected.\n\tExpected: {:?}\n\tGot: {:?}", EXPECTED_DATA, output_buffer
            );
        }
    }
}