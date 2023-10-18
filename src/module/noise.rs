extern crate rand;

use super::{common::{SignalOutputModule, OutputInfo}, ModuleManager};

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

impl SignalOutputModule for NoiseGenerator {
    fn fill_output_buffer(&mut self, data: &mut [f32], _output_info: &OutputInfo, _manager: &ModuleManager) {
        for datum in data.iter_mut() {
            *datum = self.get();
        }
    }
}