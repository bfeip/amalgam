extern crate synth;

use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use synth::module::common::{IntoMutexPtr, SignalOutputModule, MutexPtr};
use synth::module;
use synth::note::Note;

#[derive(Clone)]
struct ExampleVoice {
    osc: MutexPtr<module::oscillator::Oscillator>,
    active: bool
}

impl ExampleVoice {
    fn new() -> Self {
        let osc = Arc::new(Mutex::new(module::oscillator::Oscillator::new()));
        Self { osc, active: false }
    }
}

impl module::voice::Voice for ExampleVoice {
    fn on_activate(&mut self, note: Note) {
        self.osc.lock().expect("Osc lock is poisoned").set_frequency(note.to_freq());
        self.active = true;
    }

    fn on_start_deactivate(&mut self) {
        self.active = false;
    }

    fn fully_deactivated(&self) -> bool {
        !self.active
    }

    fn update(&mut self, reference: &Self) {
        let ref_osc = reference.osc.lock().expect("Reference Osc lock is poisoned");
        let mut osc = self.osc.lock().expect("Osc lock is poisoned");

        let ref_state = ref_osc.get_state();
        let mut state = osc.get_state_mut();
        state.pulse_width = ref_state.pulse_width;
        state.waveform = ref_state.waveform;
    }

    fn get_end_module(&mut self) -> MutexPtr<dyn SignalOutputModule> {
        self.osc.clone()
    }
}


pub fn get_repo_root() -> PathBuf {
    let mut cur_dir = env::current_dir().expect("Couldn't get working dir?");
    loop {
        let contents = match fs::read_dir(&cur_dir) {
            Ok(contents) => contents,
            Err(_err) => {
                panic!("Failed to read contents of {}", cur_dir.display());
            }
        };
        for dir_item in contents {
            if dir_item.is_err() {
                continue;
            }
            let item_name = dir_item.unwrap().file_name();
            if item_name.to_str().unwrap() == "Cargo.toml" {
                return cur_dir;
            }
        }
        if cur_dir.pop() == false {
            panic!("Failed to find repo root");
        }
    }
}


pub fn get_test_midi_file_path() -> PathBuf {
    let test_midi_file_path_from_root: PathBuf = ["data", "never_gonna_give_you_up.mid"].iter().collect();
    let repo_root = get_repo_root();
    repo_root.join(test_midi_file_path_from_root)
}

fn main() -> synth::SynthResult<()> {
    let midi_file_path = get_test_midi_file_path();
    let midi_base_module = match module::midi::MidiModuleBase::open(midi_file_path) {
        Ok(midi_base_module) => midi_base_module,
        Err(err) => {
            let msg = format!("Failed to create MIDI base module: {}", err);
            return Err(synth::SynthError::new(&msg));
        }
    };

    // TODO: There's a lot of Arc<Mutex<T>> creation. Maybe they should get wrapped into an object 
    let midi_note_output = module::midi::midi_note::MidiNoteOutput::new(
        midi_base_module.into_mutex_ptr()
    );
    let note_source = Arc::new(Mutex::new(midi_note_output));
    let reference_voice = Arc::new(Mutex::new(ExampleVoice::new()));
    let voice_set = module::voice::VoiceSet::new(reference_voice, 24, note_source);

    let mut example_synth = match synth::Synth::new() {
        Ok(synth) => synth,
        Err(err) => {
            let msg = format!("Failed to create synth: {}", err);
            return Err(synth::SynthError::new(&msg));
        }
    };
    example_synth.get_output_module_mut().set_audio_input(Box::new(voice_set));

    // TODO: it's annoying to have to crete this as a user. It should be created by the synth
    let mut audio_output = match synth::output::AudioOutput::new(synth::output::OutputDeviceType::Cpal) {
        Ok(audio_output) => audio_output,
        Err(err) => {
            let msg = format!("Failed to example output: failed to create audio output: {}", err);
            return Err(synth::SynthError::new(&msg));
        }
    };

    let synth_mutex_ptr = std::sync::Arc::new(std::sync::Mutex::new(example_synth));

    if let Err(err) = synth::Synth::play(synth_mutex_ptr, &mut audio_output) {
        let msg = format!("Failed to test full synth: {}", err);
        return Err(synth::SynthError::new(&msg));
    }

    // Wait forever
    loop { std::thread::yield_now(); }
}