use crate::note;
use super::common::*;
use super::empty::OptionalEmpty;

const PI: f32 = std::f64::consts::PI as f32;
const TAU: f32 = PI * 2.0;
const U16_MID: u16 = u16::MAX / 2;

/// Represents one of the basic waveforms
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Waveform {
    Sine,
    Triangle,
    Saw,
    Ramp,
    Pulse
}

/// Represents a Oscillator capable of outputting values
#[derive(Clone)]
pub struct Oscillator {
    /// Basic waveform that will be played
    waveform: Waveform,
    /// frequency in Hz that the wave will be played at
    frequency: f32,
    /// Width of the pulse. Only used for pulse waveforms. 50% is square, 0% and 100% are silent
    pulse_width: f32,
    freq_override_input: Connectable<dyn OptionalSignalOutputModule>
}

impl Oscillator {
    /// Creates a basic sine wave oscillator stream with a default `OscillatorState`
    pub fn new() -> Self {
        let waveform = Waveform::Sine;
        let frequency = note::FREQ_C;
        let pulse_width = 0.5;
        let freq_override_input = OptionalEmpty::new().into();
        Oscillator { waveform, frequency, pulse_width, freq_override_input }
    }

    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.waveform = waveform
    }

    pub fn get_waveform(&self) -> Waveform {
        self.waveform
    }

    pub fn set_frequency(&mut self, freq: f32) {
        self.frequency = freq
    }

    pub fn get_frequency(&self) -> f32 {
        self.frequency
    }

    pub fn set_pulse_width(&mut self, pulse_width: f32) {
        self.pulse_width = pulse_width;
    }

    pub fn get_pulse_width(&self) -> f32 {
        self.pulse_width
    }

    pub fn set_frequency_override_input(
        &mut self, override_input: Connectable<dyn OptionalSignalOutputModule>
    ) {
        self.freq_override_input = override_input;
    }

    fn fill_sine(&self, buffer: &mut [f32], clock_values: &[usize], freq_values: &[f32], sample_rate: usize) {
        let buffer_len = buffer.len();
        debug_assert!(buffer_len == clock_values.len() && buffer_len == freq_values.len());
        for i in 0..buffer_len {
            let clock_value = clock_values[i] as f32;
            let freq_value = freq_values[i];
            let sample_rate = sample_rate as f32;

            buffer[i] = (freq_value * clock_value * TAU / sample_rate).sin();
        }
    }

    fn fill_ramp(&self, buffer: &mut [f32], clock_values: &[usize], freq_values: &[f32], sample_rate: usize) {
        let buffer_len = buffer.len();
        debug_assert!(buffer_len == clock_values.len() && buffer_len == freq_values.len());
        for i in 0..buffer_len {
            let clock_value = clock_values[i] as f32;
            let freq_value = freq_values[i];
            let sample_rate = sample_rate as f32;

            buffer[i] = (freq_value * clock_value * 2_f32 / sample_rate) % 2_f32 - 1_f32;
        }
    }

    fn fill_saw(&self, buffer: &mut [f32], clock_values: &[usize], freq_values: &[f32], sample_rate: usize) {
        let buffer_len = buffer.len();
        debug_assert!(buffer_len == clock_values.len() && buffer_len == freq_values.len());
        for i in 0..buffer_len {
            let clock_value = clock_values[i] as f32;
            let freq_value = freq_values[i];
            let sample_rate = sample_rate as f32;

            buffer[i] = (freq_value * clock_value * -2_f32 / sample_rate) % 2_f32 + 1_f32;
        }
    }

    fn fill_pulse(&self, buffer: &mut [f32], clock_values: &[usize], freq_values: &[f32], sample_rate: usize) {
        let buffer_len = buffer.len();
        debug_assert!(buffer_len == clock_values.len() && buffer_len == freq_values.len());
        for i in 0..buffer_len {
            let clock_value = clock_values[i] as f32;
            let freq_value = freq_values[i];
            let sample_rate = sample_rate as f32;

            let duration_offset = (clock_value * freq_value / sample_rate) % 1_f32;
            buffer[i] = if duration_offset > self.pulse_width { 1_f32 } else { -1_f32 };
        }
    }

    fn fill(
        &self, buffer: &mut [f32], clock_values: &[usize],
        freq_override_buffer: &[Option<f32>], sample_rate: usize
    ) {
        let buffer_len = buffer.len();

        // Compute frequency per sample
        let mut freq_values = vec![self.frequency; buffer_len];
        for (freq_value, &freq_override_elem) in freq_values.iter_mut().zip(freq_override_buffer.iter()) {
            if freq_override_elem.is_some() {
                *freq_value = freq_override_elem.unwrap();
            }
        }

        match self.waveform {
            Waveform::Sine     => self.fill_sine(buffer, clock_values, &freq_values, sample_rate),
            Waveform::Ramp     => self.fill_ramp(buffer, clock_values, &freq_values, sample_rate),
            Waveform::Saw      => self.fill_saw(buffer, clock_values, &freq_values, sample_rate),
            Waveform::Pulse    => self.fill_pulse(buffer, clock_values, &freq_values, sample_rate),
            Waveform::Triangle => todo!()
        }
    }
}

impl SignalOutputModule for Oscillator {
    fn fill_output_buffer(&mut self, data: &mut [f32], output_info: &OutputInfo) {
        let buffer_len = data.len();

        // Get freq override input
        let mut freq_override_buffer = vec![None; buffer_len];
        let mut freq_override_module = self.freq_override_input.lock();
        freq_override_module.fill_optional_output_buffer(freq_override_buffer.as_mut_slice(), output_info);

        self.fill(data, &output_info.current_sample_range, &freq_override_buffer, output_info.sample_rate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use crate::clock;

    fn get_osc_data(
        osc: &mut Oscillator,
        data_size: usize,
        sample_rate: usize,
    ) -> Vec<f32> {
        let mut clock = clock::SampleClock::new(sample_rate);
        let clock_values = clock.get_range(data_size);
        let output_info = OutputInfo::new_basic(sample_rate, clock_values);

        let mut data = Vec::with_capacity(data_size);
        data.resize(data_size, 0_f32);
        osc.fill_output_buffer(&mut data, &output_info);

        data
    }

    #[test]
    fn test_sine() {
        const EXPECTED_DATA: &[f32] = &[1.0, 0.0, -1.0, 0.0];
        let mut osc = Oscillator::new();
        osc.set_waveform(Waveform::Sine);
        osc.set_frequency(1_f32);
        let data = get_osc_data(&mut osc, 4, 4);

        for i in 0..4 {
            if !float_eq(EXPECTED_DATA[i], data[i], 0.001) {
                panic!(
                    "Oscillator output differs from expected:\n\tExpected: {:?},\n\tGot: {:?}",
                    EXPECTED_DATA, data
                );
            }
        }
    }

    #[test]
    fn test_ramp() {
        const EXPECTED_DATA: &[f32] = &[-0.5, 0.0, 0.5, -1.0];
        let mut osc = Oscillator::new();
        osc.set_frequency(1_f32);
        osc.set_waveform(Waveform::Ramp);
        let data = get_osc_data(&mut osc, 4, 4);

        for i in 0..4 {
            if !float_eq(EXPECTED_DATA[i], data[i], 0.001) {
                panic!(
                    "Oscillator output differs from expected:\n\tExpected: {:?},\n\tGot: {:?}",
                    EXPECTED_DATA, data
                );
            }
        }
    }

    #[test]
    fn test_saw() {
        const EXPECTED_DATA: &[f32] = &[0.5, 0.0, -0.5, 1.0];
        let mut osc = Oscillator::new();
        osc.set_waveform(Waveform::Saw);
        osc.set_frequency(1_f32);
        let data = get_osc_data(&mut osc, 4, 4);

        for i in 0..4 {
            if !float_eq(EXPECTED_DATA[i], data[i], 0.001) {
                panic!(
                    "Oscillator output differs from expected:\n\tExpected: {:?},\n\tGot: {:?}",
                    EXPECTED_DATA, data
                );
            }
        }
    }

    #[test]
    fn test_square() {
        const EXPECTED_DATA: &[f32] = &[-1.0, -1.0, -1.0, -1.0, -1.0, 1.0, 1.0, 1.0, 1.0, -1.0];
        let mut osc = Oscillator::new();
        osc.set_waveform(Waveform::Pulse);
        osc.set_pulse_width(0.5);
        osc.set_frequency(1_f32);
        let data = get_osc_data(&mut osc, 10, 10);

        for i in 0..10 {
            if !float_eq(EXPECTED_DATA[i], data[i], 0.001) {
                panic!(
                    "Oscillator output differs from expected:\n\tExpected: {:?},\n\tGot: {:?}",
                    EXPECTED_DATA, data
                );
            }
        }
    }

    #[test]
    fn test_25_pulse() {
        const EXPECTED_DATA: &[f32] = &[-1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, -1.0];
        let mut osc = Oscillator::new();
        osc.set_waveform(Waveform::Pulse);
        osc.set_pulse_width(0.25);
        osc.set_frequency(1_f32);
        let data = get_osc_data(&mut osc, 10, 10);

        for i in 0..10 {
            if !float_eq(EXPECTED_DATA[i], data[i], 0.001) {
                panic!(
                    "Oscillator output differs from expected:\n\tExpected: {:?},\n\tGot: {:?}",
                    EXPECTED_DATA, data
                );
            }
        }
    }

    #[test]
    fn test_75_pulse() {
        const EXPECTED_DATA: &[f32] = &[-1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, 1.0, 1.0, -1.0];
        let mut osc = Oscillator::new();
        osc.set_waveform(Waveform::Pulse);
        osc.set_pulse_width(0.75);
        osc.set_frequency(1_f32);
        let data = get_osc_data(&mut osc, 10, 10);

        for i in 0..10 {
            if !float_eq(EXPECTED_DATA[i], data[i], 0.001) {
                panic!(
                    "Oscillator output differs from expected:\n\tExpected: {:?},\n\tGot: {:?}",
                    EXPECTED_DATA, data
                );
            }
        }
    }
}