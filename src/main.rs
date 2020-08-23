#![allow(dead_code)]

mod prelude;
mod note;
mod clock;

use prelude::*;

const SAMPLES_PER_SECOND: Unsigned = 60;
const SAMPLE_SIZE: usize = 1024;
const PI: Float = std::f64::consts::PI as Float;
const TAU: Float = PI * 2.0;

type Sample = [Float; SAMPLE_SIZE];

#[derive(Copy, Clone, PartialEq)]
enum Waveform {
    Sine,
    Triangle,
    Saw,
    Ramp,
    Pulse
}

struct OscillatorState {
    waveform: Waveform,
    frequency: Float,
    pulse_width: Float
}

impl OscillatorState {
    fn new() -> Self {
        Self {
            waveform: Waveform::Sine,
            frequency: note::FREQ_A,
            pulse_width: 0.5
        }
    }
}

struct OscillatorStream {
    state: OscillatorState,
    clock: clock::Clock
}

impl OscillatorStream {
    fn new() -> Self {
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

    fn fill_sample(&self, sample: &mut Sample) {
        match self.state.waveform {
            Waveform::Sine     => self.fill_sample_sine(sample),
            Waveform::Triangle => todo!(),
            Waveform::Saw      => todo!(),
            Waveform::Ramp     => todo!(),
            Waveform::Pulse    => todo!()
        }
    }

    fn fill_sample_sine(&self, sample: &mut Sample) {
        let freq = self.state.frequency;
        let start_offset = self.clock.get_nanoseconds();
        for i in 0..SAMPLE_SIZE {
            
        }
    }
}

fn main() {
    println!("Hello, world!");
}
