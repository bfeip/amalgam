extern crate rand;

use super::{SynthModule, OutputInfo};

pub struct NoiseGenerator {
}

impl NoiseGenerator {
    pub fn new() -> NoiseGenerator {
        NoiseGenerator {}
    }

    pub fn get<T>(&self) -> T 
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