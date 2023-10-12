pub mod common;
pub mod error;

mod sample_buffer;
mod compressor;
mod attenuverter;
mod noise;
mod oscillator;
mod sequencer;
mod mixer;
mod envelope;
mod midi;
mod output;
//mod voice;

pub use compressor::Compressor;
pub use attenuverter::Attenuverter;
pub use noise::NoiseGenerator;
pub use oscillator::Oscillator;
pub use sequencer::Sequencer;
pub use mixer::Mixer;
pub use envelope::Envelope;
pub use midi::MidiModuleBase;
//pub use midi::midi_note::MidiNoteOutput;
pub use output::Output;

use std::collections::HashMap;

use self::common::SignalOutputModule;

pub const NULL_KEY: ModuleKey = 0;

type ModuleKey = usize;

enum Module {
    Oscillator(Oscillator)
}

impl SignalOutputModule for Module {
    fn fill_output_buffer(&mut self, _buffer: &mut [f32], _output_info: &common::OutputInfo, _manager: &mut ModuleManager) {
        todo!();
    }
}

pub struct ModuleManager {
    modules: HashMap<ModuleKey, Module>,
    next_key: ModuleKey
}

impl ModuleManager {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            next_key: 1
        }
    }

    pub fn add(&mut self, module: Module) -> ModuleKey {
        self.modules.insert(self.next_key, module);
        let ret = self.next_key;
        self.next_key += 1;
        ret
    }

    pub fn get(&self, key: ModuleKey) -> Option<&Module> {
        self.modules.get(&key)
    }

    pub fn get_mut(&mut self, key: ModuleKey) -> Option<&mut Module> {
        self.modules.get_mut(&key)
    }

    pub fn remove(&mut self, key: ModuleKey) -> Option<Module> {
        self.modules.remove(&key)
    }
}