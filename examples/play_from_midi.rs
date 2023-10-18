extern crate amalgam;

use std::env;
use std::fs;
use std::path::PathBuf;

use amalgam::module::Compressor;
use amalgam::module::{MidiModuleBase, MidiNoteOutput, Attenuverter, Envelope, Oscillator, Voice, VoiceSet};
use amalgam::module::common::*;
use amalgam::{note, output, Synth};
use amalgam::error::*;

#[derive(Clone)]
struct OscillatorFrequencyOverride {
    freqs: Vec<Option<f32>>,
    previous_freq: f32
}

impl OscillatorFrequencyOverride {
    fn new() -> Self {
        Self { 
            freqs: Vec::new(),
            previous_freq: 0.0
        }
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
            if freq.is_some() {
                *buffer_val = freq;
                self.previous_freq = freq.unwrap();
            }
            else {
                *buffer_val = Some(self.previous_freq);
            }
        }
    }
}

#[derive(Clone)]
struct EnvelopeTrigger {
    starts: Vec<usize>,
    ends: Vec<usize>,
    trigger_signal: f32
}

impl EnvelopeTrigger {
    fn new() -> Self {
        let starts = Vec::new();
        let ends = Vec::new();
        let trigger_signal = 0.0;
        Self { starts, ends, trigger_signal }
    }

    fn clear(&mut self) {
        self.starts.clear();
        self.ends.clear();
    }

    fn add_start(&mut self, start: usize) {
        self.starts.push(start);
    }

    fn add_end(&mut self, end: usize) {
        self.ends.push(end);
    }
}

impl SynthModule for EnvelopeTrigger {
    fn fill_output_buffer(&mut self, buffer: &mut[f32], _output_info: &OutputInfo) {
        let buffer_len = buffer.len();
        for i in 0..buffer_len {
            if self.starts.contains(&i) {
                self.trigger_signal = 1.0;
            }
            if self.ends.contains(&i) {
                self.trigger_signal = 0.0;
            }

            buffer[i] = self.trigger_signal;
        }
    }
}

struct ExampleVoice {
    voice_number: Option<usize>,
    osc: Connectable<Oscillator>,
    env: Connectable<Envelope>,
    atten: Connectable<Attenuverter>,
    freq_override: Connectable<OscillatorFrequencyOverride>,
    env_trigger: Connectable<EnvelopeTrigger>
}

impl ExampleVoice {
    fn new() -> Self {
        let voice_number = None;

        // Set up oscillator
        let mut unconnected_osc = Oscillator::new();
        let freq_override: Connectable<OscillatorFrequencyOverride> = OscillatorFrequencyOverride::new().into();
        unconnected_osc.set_frequency_override_input(freq_override.clone().into());
        let osc: Connectable<Oscillator> = unconnected_osc.into();

        // Set up envelope
        let mut unconnected_env = Envelope::new();
        let env_trigger: Connectable<EnvelopeTrigger> = EnvelopeTrigger::new().into();
        unconnected_env.set_attack_time(500_f32);
        unconnected_env.set_release_time(100_f32);
        unconnected_env.set_trigger(env_trigger.clone().into());
        let env: Connectable<Envelope> = unconnected_env.into();

        // Set up attenuverter
        let mut unconnected_atten = Attenuverter::new();
        unconnected_atten.set_control_in(env.clone().into());
        unconnected_atten.set_signal_in(osc.clone().into());
        let atten: Connectable<Attenuverter> = unconnected_atten.into();

        Self { voice_number, osc, env, atten, freq_override, env_trigger }
    }
}

impl Voice for ExampleVoice {
    fn update(&mut self, reference: &Self) {
        let ref_osc = reference.osc.lock();
        let mut osc = self.osc.lock();
        osc.set_pulse_width(ref_osc.get_pulse_width());
        osc.set_waveform(ref_osc.get_waveform());

        let ref_env = reference.env.lock();
        let mut env = self.env.lock();
        env.copy_state_from(&ref_env);

        let ref_atten = reference.atten.lock();
        let mut atten = self.atten.lock();
        atten.copy_state_from(&ref_atten);
    }

    fn fill_output_for_note_intervals(
        &mut self, sample_buffer: &mut [f32], intervals: &[note::NoteInterval],
        output_info: &OutputInfo, voice_number: usize
    ) {
        let buffer_len = sample_buffer.len();

        debug_assert!(
            *self.voice_number.get_or_insert(voice_number) == voice_number,
            "Expected voice number to remain consistent"
        );

        // Don't forget to clear out the starts and ends for the envelope
        self.env_trigger.lock().clear();

        // Get freq value for each sample
        let mut freq_values = Vec::with_capacity(buffer_len);
        let mut sample_counter = 0_usize;
        for note_interval in intervals {
            // Some checks that note intervals appear in the expected order and do not overlap one another
            let is_correct_start_note = note_interval.start.is_none() && sample_counter == 0;
            let is_ordered = note_interval.start.unwrap_or(sample_counter) >= sample_counter;
            debug_assert!(is_correct_start_note || is_ordered, "Overlapping intervals");
            debug_assert!(sample_counter < buffer_len, "Too many samples");

            // If start sample is None that means the note started in a previous sample period and we enter this
            // sample period with the note already playing
            if note_interval.start.is_some() {
                let start = note_interval.start.unwrap();
                self.env_trigger.lock().add_start(start);
                while sample_counter != start {
                    // Until the next note is played just push nothing
                    freq_values.push(None);
                    sample_counter += 1;
                }
            }

            if note_interval.end.is_some() {
                let end = note_interval.end.unwrap();
                self.env_trigger.lock().add_end(end);
            }

            let end_sample = note_interval.end.unwrap_or(buffer_len);
            let note_freq = note_interval.note.to_freq();
            while sample_counter != end_sample {
                // Until the note is done playing, push the note's freq value
                freq_values.push(Some(note_freq));
                sample_counter += 1;
            }
        }
        
        // We're done with the intervals, wrap things up.
        while sample_counter < buffer_len {
            freq_values.push(None);
            sample_counter += 1;
        }
        {
            // Lock and set values
            let mut freq_override = self.freq_override.lock();
            freq_override.set(freq_values.clone());
        }

        self.atten.lock().fill_output_buffer(sample_buffer, output_info);
    }
}

// This deep clone method is needed so that when the voices are created they are not referencing the same components.
// In the future we may want to have this be a method that Voice implementors have to implement e.g. `deep_clone()`.
impl Clone for ExampleVoice {
    fn clone(&self) -> Self {
        let voice_number = self.voice_number;

        let mut unconnected_osc = self.osc.lock().clone();
        let mut unconnected_env = self.env.lock().clone();
        let mut unconnected_atten = self.atten.lock().clone();
        let freq_override: Connectable<OscillatorFrequencyOverride> = self.freq_override.lock().clone().into();
        let env_trigger: Connectable<EnvelopeTrigger> = self.env_trigger.lock().clone().into();

        unconnected_osc.set_frequency_override_input(freq_override.clone().into());

        unconnected_env.set_trigger(env_trigger.clone().into());

        let osc: Connectable<Oscillator> = unconnected_osc.into();
        let env: Connectable<Envelope> = unconnected_env.into();
        unconnected_atten.set_signal_in(osc.clone().into());
        unconnected_atten.set_control_in(env.clone().into());
        let atten = unconnected_atten.into();

        Self {
            voice_number,
            osc,
            env,
            atten,
            freq_override,
            env_trigger
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

fn main() -> SynthResult<()> {
    let midi_file_path = get_test_midi_file_path();
    let mut midi_base_module = match MidiModuleBase::open(midi_file_path) {
        Ok(midi_base_module) => midi_base_module,
        Err(err) => {
            let msg = format!("Failed to create MIDI base module: {}", err);
            return Err(SynthError::new(&msg));
        }
    };

    // Set track to 1, which is where the actual notes are in this MIDI file
    if let Err(err) = midi_base_module.set_track(1) {
        let msg = format!("Failed to set correct MIDI track to read from: {}", err);
        return Err(SynthError::new(&msg));
    }
    
    let midi_note_output = MidiNoteOutput::new(
        midi_base_module.into()
    );

    let note_source: Connectable<dyn NoteOutputModule> = midi_note_output.into();
    let reference_voice = ExampleVoice::new().into();

    let voice_set = VoiceSet::new(reference_voice, 5, note_source);

    let mut compressor = Compressor::new();
    compressor.set_signal_in(voice_set.into());

    let mut example_synth = match Synth::new() {
        Ok(synth) => synth,
        Err(err) => {
            let msg = format!("Failed to create synth: {}", err);
            return Err(SynthError::new(&msg));
        }
    };
    let output_module = example_synth.get_output_module_mut();
    output_module.set_audio_input(compressor.into());

    // TODO: it's annoying to have to create this as a user. It should be created by the synth
    let mut audio_output = match output::AudioOutput::new(output::OutputDeviceType::Cpal) {
        Ok(audio_output) => audio_output,
        Err(err) => {
            let msg = format!("Failed to example output: failed to create audio output: {}", err);
            return Err(SynthError::new(&msg));
        }
    };

    let synth_mutex_ptr = std::sync::Arc::new(std::sync::Mutex::new(example_synth));

    if let Err(err) = Synth::play(synth_mutex_ptr, &mut audio_output) {
        let msg = format!("Failed to test full synth: {}", err);
        return Err(SynthError::new(&msg));
    }

    // Wait forever
    loop { std::thread::yield_now(); }

    // std::thread::sleep(Duration::from_secs(6));
    // Ok(())
}