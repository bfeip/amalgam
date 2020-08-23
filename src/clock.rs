use std::time::{Duration, Instant};

pub struct Clock {
    start: Instant
}

impl Clock {
    pub fn new() -> Self {
        let start = Instant::now();
        Clock { start }
    }

    pub fn get_duration(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn get_nanoseconds(&self) -> u128 {
        self.start.elapsed().as_nanos()
    }
}