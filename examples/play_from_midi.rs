extern crate synth;

use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use synth::module::common::*;
use synth::module;

struct OscillatorFrequencyOverride {
    freqs: Vec<Option<f32>>
}

impl OscillatorFrequencyOverride {
    fn new(freqs: Vec<Option<f32>>) -> Self {
        Self { freqs }
    }

    fn set(&mut self, freqs: Vec<Option<f32>>) {
        self.freqs = freqs;
    }
}

impl OptionalSignalOutputModule for OscillatorFrequencyOverride {
    fn fill_optional_output_buffer(&mut self, buffer: &mut[Option<f32>], _output_info: &OutputInfo) {
        let buffer_len = buffer.len();
        assert!(buffer_len == self.freqs.len(), "Mismatched buffers");
        for (buffer_val, &freq) in buffer.iter_mut().zip(self.freqs.iter()) {
            *buffer_val = freq;
        }
    }
}

#[derive(Clone)]
struct ExampleVoice {
    osc: MutexPtr<module::oscillator::Oscillator>,
    freq_override: MutexPtr<OscillatorFrequencyOverride>
}

impl ExampleVoice {
    fn new() -> Self {
        let mut unmutexed_osc = module::oscillator::Oscillator::new();
        let freq_override = Arc::new(Mutex::new(OscillatorFrequencyOverride::new(Vec::new())));
        unmutexed_osc.set_frequency_override_input(freq_override.clone());
        let osc = Arc::new(Mutex::new(unmutexed_osc));
        Self { osc, freq_override }
    }
}

impl module::voice::Voice for ExampleVoice {
    fn update(&mut self, reference: &Self) {
        let ref_osc = reference.osc.lock().expect("Reference Osc lock is poisoned");
        let mut osc = self.osc.lock().expect("Osc lock is poisoned");

        let ref_state = ref_osc.get_state();
        let mut state = osc.get_state_mut();
        state.pulse_width = ref_state.pulse_width;
        state.waveform = ref_state.waveform;
    }

    fn fill_output_for_note_intervals(
        &mut self, sample_buffer: &mut [f32], intervals: &[module::voice::NoteInterval],
        output_info: &module::common::OutputInfo
    ) {
        assert!(!intervals.is_empty(), "Tried to get output with no note intervals");
        let buffer_len = sample_buffer.len();

        // Get freq value for each sample
        let mut freq_values = Vec::with_capacity(buffer_len);
        let mut sample_counter = 0_usize;
        for note_interval in intervals {
            // Some checks that note intervals appear in the expected order and do not overlap one another
            let is_correct_start_note = note_interval.start_sample.is_none() && sample_counter == 0;
            let is_ordered = note_interval.start_sample.unwrap_or(sample_counter) >= sample_counter;
            assert!(is_correct_start_note || is_ordered, "Overlapping intervals");
            assert!(sample_counter < buffer_len, "Too many samples");

            // If start sample is None that means the note started in a previous sample period and we enter this
            // sample period with the note already playing
            if note_interval.start_sample.is_some() {
                while sample_counter != note_interval.start_sample.unwrap() {
                    // Until the next note is played just push nothing
                    freq_values.push(None);
                    sample_counter += 1;
                }
            }

            let end_sample = note_interval.end_sample.unwrap_or(buffer_len);
            while sample_counter != end_sample {
                // Until the note is done playing, push the notes freq value
                let note_freq = note_interval.note.to_freq();
                let end_sample = note_interval.end_sample.unwrap_or(buffer_len);
                while sample_counter != end_sample {
                    // Until the note is done playing, push the notes freq value
                    freq_values.push(Some(note_freq));
                    sample_counter += 1;
                }
            }
            while sample_counter < buffer_len {
                freq_values.push(None);
                sample_counter += 1;
            }
            {
                // Lock and set values
                let mut freq_override = self.freq_override.lock().expect("Lock is poisoned");
                freq_override.set(freq_values.clone());
            }

            self.osc.lock().unwrap().fill_output_buffer(sample_buffer, output_info);
            for (sample, &freq) in sample_buffer.iter_mut().zip(freq_values.iter()) {
                if freq.is_none() {
                    // Do not play if no note was active
                    // This is kinda a simple envelope generator
                    *sample = 0.0;
                }
            }
        }
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
    let test_midi_file_path_from_root: PathBuf = ["data", "basic_test.mid"].iter().collect();
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