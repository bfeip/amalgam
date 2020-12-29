use crate::note;
use crate::clock;
use super::traits::SignalOutputModule;

const PI: f32 = std::f64::consts::PI as f32;
const TAU: f32 = PI * 2.0;
const U16_MID: u16 = u16::MAX / 2;

/// Represents one of the basic waveforms
#[derive(Copy, Clone, PartialEq)]
pub enum Waveform {
    Sine,
    Triangle,
    Saw,
    Ramp,
    Pulse
}

/// Contains the parameters that determine how an oscillator will sound
#[derive(Copy, Clone)]
pub struct OscillatorState {
    /// Basic waveform that will be played
    pub waveform: Waveform,
    /// frequency in Hz that the wave will be played at
    pub frequency: f32,
    /// Width of the pulse. Only used for pulse waveforms. 50% is square, 0% and 100% are silent
    pub pulse_width: f32,
    /// The sample rate of the output
    pub sample_rate: f32
}

impl OscillatorState {
    /// Creates a basic sine wave oscillator state in C4 with a 50% pulse width
    pub fn new(sample_rate: f32) -> Self {
        Self {
            waveform: Waveform::Sine,
            frequency: note::FREQ_C,
            pulse_width: 0.5,
            sample_rate
        }
    }
}

/// Represents a Oscillator capable of outputting values
#[derive(Clone)]
pub struct Oscillator {
    /// Contains all the information needed to replicate the oscillator
    state: OscillatorState,
    /// A timer element used to get the the wave state at any instant
    clock: clock::SampleClock
}

impl Oscillator {
    /// Creates a basic sine wave oscillator stream with a default `OscillatorState`
    pub fn new(sample_rate: f32) -> Self {
        let state = OscillatorState::new(sample_rate);
        let clock = clock::SampleClock::new(sample_rate);
        Oscillator { state, clock }
    }

    pub fn from_state(state: &OscillatorState) -> Self {
        let clock = clock::SampleClock::new(state.sample_rate);
        Oscillator { state: state.clone(), clock }
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
        self.state = *state;
    }

    // UNIT OUTPUTS
    /// Gets a sine wave output as a `f32` bound between -1.0 and 1.0
    #[inline]
    pub fn get_sine(&mut self) -> f32 {
        (self.state.frequency * self.clock.get() * TAU / self.state.sample_rate).sin()
    }

    #[inline]
    /// Gets a ramp wave output as a `f32` bound between -1.0 and 1.0
    pub fn get_ramp(&mut self) -> f32 {
        (self.clock.get() * self.state.frequency * 2_f32 / self.state.sample_rate) % 2_f32 - 1_f32
    }

    #[inline]
    /// Gets a saw wave output as a `f32` bound between -1.0 and 1.0
    pub fn get_saw(&mut self) -> f32 {
        self.get_ramp() * -1_f32
    }

    #[inline]
    /// Gets a pulse wave output as a `f32` bound between -1.0 and 1.0
    pub fn get_pulse(&mut self) -> f32 {
        let duration_offset = (self.clock.get() * self.state.frequency / self.state.sample_rate) % 1_f32;
        if duration_offset > self.state.pulse_width { 1_f32 } else { -1_f32 }
    }

    /// Gets whatever wave is indicated by the OscillatorState as a `f32` bound between -1.0 and 1.0
    pub fn get(&mut self) -> f32 {
        match self.state.waveform {
            Waveform::Sine     => self.get_sine(),
            Waveform::Ramp     => self.get_ramp(),
            Waveform::Saw      => self.get_saw(),
            Waveform::Pulse    => self.get_pulse(),
            Waveform::Triangle => todo!()
        }
    }
}

impl SignalOutputModule for Oscillator {
    fn fill_output_buffer(&mut self, data: &mut [f32]) {
        for datum in data.iter_mut() {
            *datum = self.get();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    fn get_osc_data_with_state(state: &OscillatorState, data_size: usize) -> Vec<f32> {
        let mut osc = Oscillator::from_state(state);
        osc.set_state(state);

        let mut data = Vec::with_capacity(data_size);
        data.resize(data_size, 0_f32);
        osc.fill_output_buffer(&mut data);

        data
    }

    #[test]
    fn test_sine() {
        const EXPECTED_DATA: &[f32] = &[1.0, 0.0, -1.0, 0.0];
        let mut osc_state = OscillatorState::new(4_f32);
        osc_state.waveform = Waveform::Sine;
        osc_state.frequency = 1_f32;
        let data = get_osc_data_with_state(&osc_state, 4);

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
        let mut osc_state = OscillatorState::new(4_f32);
        osc_state.waveform = Waveform::Ramp;
        osc_state.frequency = 1_f32;
        let data = get_osc_data_with_state(&osc_state, 4);

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
        let mut osc_state = OscillatorState::new(4_f32);
        osc_state.waveform = Waveform::Saw;
        osc_state.frequency = 1_f32;
        let data = get_osc_data_with_state(&osc_state, 4);

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
        let mut osc_state = OscillatorState::new(10_f32);
        osc_state.waveform = Waveform::Pulse;
        osc_state.pulse_width = 0.5;
        osc_state.frequency = 1_f32;
        let data = get_osc_data_with_state(&osc_state, 10);

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
        let mut osc_state = OscillatorState::new(10_f32);
        osc_state.waveform = Waveform::Pulse;
        osc_state.pulse_width = 0.25;
        osc_state.frequency = 1_f32;
        let data = get_osc_data_with_state(&osc_state, 10);

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
        let mut osc_state = OscillatorState::new(10_f32);
        osc_state.waveform = Waveform::Pulse;
        osc_state.pulse_width = 0.75;
        osc_state.frequency = 1_f32;
        let data = get_osc_data_with_state(&osc_state, 10);

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