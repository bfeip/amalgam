use std::time::{Duration, Instant};

#[derive(Debug, Copy, Clone)]
/// A clock. It measures time intervals
pub struct Clock {
    start: Instant
}

impl Clock {
    /// Creates a new `Clock`
    pub fn new() -> Self {
        let start = Instant::now();
        Clock { start }
    }

    /// Gets the amount of time that has passed since the clock was started as a `Duration`
    pub fn get_duration(&self) -> Duration {
        self.start.elapsed()
    }

    /// Gets the number of nanoseconds that have elapsed since the clock started
    pub fn get_nanoseconds(&self) -> u128 {
        self.start.elapsed().as_nanos()
    }
}

#[derive(Copy, Clone)]
pub struct SampleClock {
    sample_rate: f32,
    value: f32
}

impl SampleClock {
    pub fn new(sample_rate: f32) -> Self {
        let value = 0_f32;
        Self { sample_rate, value }
    }

    pub fn get(&mut self) -> f32 {
        self.value = (self.value + 1_f32) % self.sample_rate;
        self.value
    }
}