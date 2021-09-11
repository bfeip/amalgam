use crate::note;
use super::common::*;
use super::empty::OptionalEmpty;

use std::sync::{Arc, Mutex};

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

/// Contains the parameters that determine how an oscillator will sound
#[derive(Debug, Clone)]
pub struct OscillatorState {
    /// Basic waveform that will be played
    pub waveform: Waveform,
    /// frequency in Hz that the wave will be played at
    pub frequency: f32,
    /// Width of the pulse. Only used for pulse waveforms. 50% is square, 0% and 100% are silent
    pub pulse_width: f32,
}

impl OscillatorState {
    /// Creates a basic sine wave oscillator state in C4 with a 50% pulse width
    pub fn new() -> Self {
        Self {
            waveform: Waveform::Sine,
            frequency: note::FREQ_C,
            pulse_width: 0.5,
        }
    }
}

/// Represents a Oscillator capable of outputting values
#[derive(Clone)]
pub struct Oscillator {
    /// Contains all the information needed to replicate the oscillator
    state: OscillatorState,
    freq_override_input: MutexPtr<dyn OptionalSignalOutputModule>
}

impl Oscillator {
    /// Creates a basic sine wave oscillator stream with a default `OscillatorState`
    pub fn new() -> Self {
        let state = OscillatorState::new();
        let freq_input = Arc::new(Mutex::new(OptionalEmpty::new()));
        Oscillator { state, freq_override_input: freq_input }
    }

    pub fn from_state(state: &OscillatorState) -> Self {
        let freq_input = Arc::new(Mutex::new(OptionalEmpty::new()));
        Oscillator { state: state.clone(), freq_override_input: freq_input }
    }

    /// Retrieves a reference to the `OscillatorState`
    pub fn get_state(&self) -> &OscillatorState {
        &self.state
    }

    /// Retrieves a mutable reference to the `OscillatorState`
    pub fn get_state_mut(&mut self) -> &mut OscillatorState {
        &mut self.state
    }

    pub fn set_state(&mut self, state: &OscillatorState) {
        self.state = state.clone();
    }

    pub fn set_frequency(&mut self, freq: f32) {
        self.state.frequency = freq
    }

    pub fn set_frequency_override_input(&mut self, override_input: MutexPtr<dyn OptionalSignalOutputModule>) {
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
            buffer[i] = if duration_offset > self.state.pulse_width { 1_f32 } else { -1_f32 };
        }
    }

    fn fill(
        &self, buffer: &mut [f32], clock_values: &[usize],
        freq_override_buffer: &[Option<f32>], sample_rate: usize
    ) {
        let buffer_len = buffer.len();

        // Compute frequency per sample
        let mut freq_values = vec![self.state.frequency; buffer_len];
        for (freq_value, &freq_override_elem) in freq_values.iter_mut().zip(freq_override_buffer.iter()) {
            if freq_override_elem.is_some() {
                *freq_value = freq_override_elem.unwrap();
            }
        }

        match self.state.waveform {
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
        let mut freq_override_module = self.freq_override_input.lock().expect("Lock is poisoned");
        let mut freq_override_buffer = vec![None; buffer_len];
        freq_override_module.fill_optional_output_buffer(freq_override_buffer.as_mut_slice(), output_info);

        self.fill(data, &output_info.current_sample_range, &freq_override_buffer, output_info.sample_rate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use crate::clock;

    fn get_osc_data_with_state(
        state: &OscillatorState,
        data_size: usize,
        sample_rate: usize,
    ) -> Vec<f32> {
        let mut osc = Oscillator::from_state(state);
        osc.set_state(state);

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
        let mut osc_state = OscillatorState::new();
        osc_state.waveform = Waveform::Sine;
        osc_state.frequency = 1_f32;
        let data = get_osc_data_with_state(&osc_state, 4, 4);

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
        let mut osc_state = OscillatorState::new();
        osc_state.waveform = Waveform::Ramp;
        osc_state.frequency = 1_f32;
        let data = get_osc_data_with_state(&osc_state, 4, 4);

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
        let mut osc_state = OscillatorState::new();
        osc_state.waveform = Waveform::Saw;
        osc_state.frequency = 1_f32;
        let data = get_osc_data_with_state(&osc_state, 4, 4);

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
        let mut osc_state = OscillatorState::new();
        osc_state.waveform = Waveform::Pulse;
        osc_state.pulse_width = 0.5;
        osc_state.frequency = 1_f32;
        let data = get_osc_data_with_state(&osc_state, 10, 10);

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
        let mut osc_state = OscillatorState::new();
        osc_state.waveform = Waveform::Pulse;
        osc_state.pulse_width = 0.25;
        osc_state.frequency = 1_f32;
        let data = get_osc_data_with_state(&osc_state, 10, 10);

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
        let mut osc_state = OscillatorState::new();
        osc_state.waveform = Waveform::Pulse;
        osc_state.pulse_width = 0.75;
        osc_state.frequency = 1_f32;
        let data = get_osc_data_with_state(&osc_state, 10, 10);

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