extern crate rand;

use super::traits::{SignalOutputModule, OutputInfo};

struct NoiseGenerator {
}

impl NoiseGenerator {
    fn new() -> NoiseGenerator {
        NoiseGenerator {}
    }

    // NOTE: This needs to be in the range of -1.0 to 1.0, so this is wrong
    fn get<T>(&mut self) -> T 
    where rand::distributions::Standard: rand::distributions::Distribution<T> {
        todo!();
        //rand::random()
    }
}

impl SignalOutputModule for NoiseGenerator {
    fn fill_output_buffer(&mut self, data: &mut [f32], _output_info: &OutputInfo) {
        for datum in data.iter_mut() {
            *datum = self.get();
        }
    }
}