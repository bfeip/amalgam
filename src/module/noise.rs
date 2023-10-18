extern crate rand;

use super::common::{SynthModule, OutputInfo};

pub struct NoiseGenerator {
}

impl NoiseGenerator {
    pub fn new() -> NoiseGenerator {
        NoiseGenerator {}
    }

    // NOTE: This needs to be in the range of -1.0 to 1.0, so this is wrong
    pub fn get<T>(&mut self) -> T 
    where rand::distributions::Standard: rand::distributions::Distribution<T> {
        todo!();
        //rand::random()
    }
}

impl SynthModule for NoiseGenerator {
    fn fill_output_buffer(&self, data: &mut [f32], _output_info: &OutputInfo) {
        for datum in data.iter_mut() {
            *datum = self.get();
        }
    }
}