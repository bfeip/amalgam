extern crate rand;

use super::traits::SignalOutputModule;

struct NoiseGenerator {
}

impl NoiseGenerator {
    fn new() -> NoiseGenerator {
        NoiseGenerator {}
    }

    fn get<T>(&mut self) -> T 
    where rand::distributions::Standard: rand::distributions::Distribution<T> {
        rand::random()
    }
}

impl SignalOutputModule for NoiseGenerator {
    fn fill_output_buffer(&mut self, data: &mut [f32]) {
        for datum in data.iter_mut() {
            *datum = self.get();
        }
    }
}