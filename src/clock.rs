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

#[derive(Debug, Copy, Clone)]
pub struct SampleClock {
    sample_rate: usize,
    value: usize
}

impl SampleClock {
    pub fn new(sample_rate: usize) -> Self {
        let value = 0;
        Self { sample_rate, value }
    }

    pub fn get(&mut self) -> usize {
        self.value = (self.value + 1) % self.sample_rate;
        self.value
    }

    pub fn get_range(&mut self, amount: usize) -> SampleRange {
        let range = SampleRange::new(self.sample_rate, self.value, amount);
        self.value = (self.value + amount) % self.sample_rate;
        range
    }
}

#[derive(Debug, Clone)]
pub struct SampleRange {
    sample_rate: usize,
    initial_value: usize,
    n_samples: usize,
}

impl SampleRange {
    fn new(sample_rate: usize, initial_value: usize, n_samples: usize) -> Self {
        Self {
            sample_rate,
            initial_value,
            n_samples,
        }
    }

    pub fn iter(&self) -> SampleRangeIter {
        SampleRangeIter { 
            sample_rate: self.sample_rate,
            initial_value: self.initial_value,
            n_samples: self.n_samples,
            samples_counted: 0_usize
        }
    }

    pub fn get_sample_rate(&self) -> usize {
        self.sample_rate
    }

    pub fn get_n_samples(&self) -> usize {
        self.n_samples
    }
}

pub struct SampleRangeIter {
    sample_rate: usize,
    initial_value: usize,
    n_samples: usize,
    samples_counted: usize,
}

impl Iterator for SampleRangeIter {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.samples_counted == self.n_samples {
            return None;
        }
        let ret = (self.initial_value + self.samples_counted) % self.sample_rate;
        self.samples_counted += 1;
        Some(ret)
    }
}
