use crate::note;
use crate::clock;

const PI: f32 = std::f64::consts::PI as f32;
const TAU: f32 = PI * 2.0;
const U16_MID: u16 = u16::MAX / 2;

#[derive(Copy, Clone, PartialEq)]
enum Waveform {
    Sine,
    Triangle,
    Saw,
    Ramp,
    Pulse
}

pub struct OscillatorState {
    waveform: Waveform,
    frequency: f32,
    pulse_width: f32
}

impl OscillatorState {
    fn new() -> Self {
        Self {
            waveform: Waveform::Sine,
            frequency: note::FREQ_C,
            pulse_width: 0.5
        }
    }
}

pub struct OscillatorStream {
    state: OscillatorState,
    clock: clock::Clock
}

impl OscillatorStream {
    pub fn new() -> Self {
        let state = OscillatorState::new();
        let clock = clock::Clock::new();
        Self { state, clock }
    }

    fn get_state(&self) -> &OscillatorState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut OscillatorState {
        &mut self.state
    }

    // UNIT OUTPUTS
    #[inline]
    fn get_sine_unit(&self) -> f32 {
        let partial_secs = self.clock.get_duration().as_secs_f32() % 1_f32;
        (self.state.frequency * partial_secs * TAU).sin()
    }

    #[inline]
    fn get_ramp_unit(&self) -> f32 {
        let secs = self.clock.get_duration().as_secs_f32();
        (secs * self.state.frequency) % 1_f32
    }

    #[inline]
    fn get_saw_unit(&self) -> f32 {
        self.get_ramp_unit() * -1_f32 + 1_f32
    }

    #[inline]
    fn get_pulse_unit(&self) -> f32 {
        let duration_offset = (self.clock.get_duration().as_secs_f32() * self.state.frequency) % 1_f32;
        if duration_offset > self.state.pulse_width { 1_f32 } else { -1_f32 }
    }

    // F32 OUTPUTS
    fn get_sine_f32(&self) -> f32 {
        self.get_sine_unit() * f32::MAX
    }

    fn get_ramp_f32(&self) -> f32 {
        self.get_ramp_unit() * f32::MAX
    }

    fn get_saw_f32(&self) -> f32 {
        self.get_saw_unit() * f32::MAX
    }

    fn get_pulse_f32(&self) -> f32 {
        let duration_offset = (self.clock.get_duration().as_secs_f32() * self.state.frequency) % 1_f32;
        if duration_offset > self.state.pulse_width { f32::MAX } else { f32::MIN }
    }

    // U16 OUTPUTS
    fn get_sine_u16(&self) -> u16 {
        let one_centered_sine = self.get_sine_unit() + 1_f32;
        (one_centered_sine * (U16_MID as f32)) as u16
    }

    // I16 OUTPUTS
    fn get_sine_i16(&self) -> i16 {
        let float_sine = self.get_sine_unit() * i16::MAX as f32;
        float_sine as i16
    }

    // SAMPLE FILLERS
    fn fill_sine_sample<T: cpal::Sample>(&self, sample: &mut T) {
        let format = T::FORMAT;
        match format {
            cpal::SampleFormat::F32 => *sample = cpal::Sample::from(&self.get_sine_unit()),
            cpal::SampleFormat::U16 => *sample = cpal::Sample::from(&self.get_sine_u16()),
            cpal::SampleFormat::I16 => *sample = cpal::Sample::from(&self.get_sine_i16())
        }
    }
}

pub fn cpal_output_sine<T: cpal::Sample>(data: &mut [T], _: &cpal::OutputCallbackInfo) {
    let oscillator = OscillatorStream::new();
    for sample in data.iter_mut() {
        oscillator.fill_sine_sample(sample);
    }
}