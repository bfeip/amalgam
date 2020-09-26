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

pub fn cpal_output_noise<T: cpal::Sample>(data: &mut [T], _: &cpal::OutputCallbackInfo) 
where rand::distributions::Standard: rand::distributions::Distribution<T> {
    let mut noise_gen = NoiseGenerator::new();
    for sample in data.iter_mut() {
        *sample = cpal::Sample::from(&noise_gen.get::<T>());
    }
}