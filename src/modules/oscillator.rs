use crate::note;
use crate::clock;

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
pub struct OscillatorState {
    /// Basic waveform that will be played
    waveform: Waveform,
    /// frequency in Hz that the wave will be played at
    frequency: f32,
    /// Width of the pulse. Only used for pulse waveforms. 50% is square, 0% and 100% are silent
    pulse_width: f32
}

impl OscillatorState {
    /// Creates a basic sine wave oscillator state in C4 with a 50% pulse width
    fn new() -> Self {
        Self {
            waveform: Waveform::Sine,
            frequency: note::FREQ_C,
            pulse_width: 0.5
        }
    }
}

/// Represents a Oscillator capable of outputting values
pub struct OscillatorStream {
    /// Contains all the information needed to replicate the oscillator
    state: OscillatorState,
    /// A timer element used to get the the wave state at any instant
    clock: clock::Clock
}

impl OscillatorStream {
    /// Creates a basic sine wave oscillator stream with a default `OscillatorState`
    pub fn new() -> Self {
        let state = OscillatorState::new();
        let clock = clock::Clock::new();
        Self { state, clock }
    }

    /// Retrieves a reference to the `OscillatorState`
    fn get_state(&self) -> &OscillatorState {
        &self.state
    }

    /// Retrieves a mutable reference to the `OscillatorState`
    fn get_state_mut(&mut self) -> &mut OscillatorState {
        &mut self.state
    }

    // UNIT OUTPUTS
    /// Gets a sine wave output as a `f32` bound between -1.0 and 1.0
    #[inline]
    fn get_sine_unit(&self) -> f32 {
        let partial_secs = self.clock.get_duration().as_secs_f32() % 1_f32;
        (self.state.frequency * partial_secs * TAU).sin()
    }

    #[inline]
    /// Gets a ramp wave output as a `f32` bound between -1.0 and 1.0
    fn get_ramp_unit(&self) -> f32 {
        let secs = self.clock.get_duration().as_secs_f32();
        (secs * self.state.frequency) % 1_f32
    }

    #[inline]
    /// Gets a saw wave output as a `f32` bound between -1.0 and 1.0
    fn get_saw_unit(&self) -> f32 {
        self.get_ramp_unit() * -1_f32 + 1_f32
    }

    #[inline]
    /// Gets a pulse wave output as a `f32` bound between -1.0 and 1.0
    fn get_pulse_unit(&self) -> f32 {
        let duration_offset = (self.clock.get_duration().as_secs_f32() * self.state.frequency) % 1_f32;
        if duration_offset > self.state.pulse_width { 1_f32 } else { -1_f32 }
    }

    // F32 OUTPUTS
    /// Gets a sine wave output as a `f32` bound between f32::MAX and f32::MIN
    fn get_sine_f32(&self) -> f32 {
        self.get_sine_unit() * f32::MAX
    }

    /// Gets a ramp wave output as a `f32` bound between f32::MAX and f32::MIN
    fn get_ramp_f32(&self) -> f32 {
        self.get_ramp_unit() * f32::MAX
    }

    /// Gets a saw wave output as a `f32` bound between f32::MAX and f32::MIN
    fn get_saw_f32(&self) -> f32 {
        self.get_saw_unit() * f32::MAX
    }

    /// Gets a pulse wave output as a `f32` bound between f32::MAX and f32::MIN
    fn get_pulse_f32(&self) -> f32 {
        let duration_offset = (self.clock.get_duration().as_secs_f32() * self.state.frequency) % 1_f32;
        if duration_offset > self.state.pulse_width { f32::MAX } else { f32::MIN }
    }

    // U16 OUTPUTS
    /// Gets a sine wave output as a `u16` bound between u16::MAX and 0
    fn get_sine_u16(&self) -> u16 {
        let one_centered_sine = self.get_sine_unit() + 1_f32;
        (one_centered_sine * (U16_MID as f32)) as u16
    }

    // I16 OUTPUTS
    /// Gets a sine wave output as a `i16` bound between i16::MAX and i16::MIN
    fn get_sine_i16(&self) -> i16 {
        let float_sine = self.get_sine_unit() * i16::MAX as f32;
        float_sine as i16
    }

    // SAMPLE FILLERS
    /// Fills a Cpal sample with a sine wave in the appropriate type
    fn fill_sine_sample<T: cpal::Sample>(&self, sample: &mut T) {
        let format = T::FORMAT;
        match format {
            cpal::SampleFormat::F32 => *sample = cpal::Sample::from(&self.get_sine_unit()),
            cpal::SampleFormat::U16 => *sample = cpal::Sample::from(&self.get_sine_u16()),
            cpal::SampleFormat::I16 => *sample = cpal::Sample::from(&self.get_sine_i16())
        }
    }
}

/// A test function that will continually output a sine wave through Cpal
pub fn cpal_output_sine<T: cpal::Sample>(data: &mut [T], _: &cpal::OutputCallbackInfo) {
    // TODO: This is incorrect since the sine wave is starting over every time this gets called
    let oscillator = OscillatorStream::new();
    for sample in data.iter_mut() {
        oscillator.fill_sine_sample(sample);
    }
}