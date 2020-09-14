extern crate rand;

use rand::Rng;
use rand::prelude::*;

struct NoiseGenerator {
    rng: ThreadRng
}

impl NoiseGenerator {
    fn new() -> NoiseGenerator {
        let rng = thread_rng();
        NoiseGenerator{ rng }
    }

    fn get<T>(&mut self) -> T 
    where rand::distributions::Standard: rand::distributions::Distribution<T> {
        self.rng.gen()
    }
}