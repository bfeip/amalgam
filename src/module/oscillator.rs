use crate::note;
use crate::clock;
use super::traits::SignalOutputModule;

const PI: f32 = std::f64::consts::PI as f32;
const TAU: f32 = PI * 2.0;
const U16_MID: u16 = u16::MAX / 2;

/// Represents one of the basic waveforms
#[derive(Copy, Clone, PartialEq)]
enum Waveform {
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
    waveform: Waveform,
    /// frequency in Hz that the wave will be played at
    frequency: f32,
    /// Width of the pulse. Only used for pulse waveforms. 50% is square, 0% and 100% are silent
    pulse_width: f32,
    /// The sample rate of the output
    sample_rate: f32
}

impl OscillatorState {
    /// Creates a basic sine wave oscillator state in C4 with a 50% pulse width
    fn new(sample_rate: f32) -> Self {
        Self {
            waveform: Waveform::Sine,
            frequency: note::FREQ_C,
            pulse_width: 0.5,
            sample_rate
        }
    }
}

/// Represents a Oscillator capable of outputting values
#[derive(Copy, Clone)]
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

    /// Retrieves a reference to the `OscillatorState`
    pub fn get_state(&self) -> &OscillatorState {
        &self.state
    }

    /// Retrieves a mutable reference to the `OscillatorState`
    pub fn get_state_mut(&mut self) -> &mut OscillatorState {
        &mut self.state
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
        let secs = self.clock.get();
        (secs * self.state.frequency / self.state.sample_rate) % 2_f32 - 1_f32
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