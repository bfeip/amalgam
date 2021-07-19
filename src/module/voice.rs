use super::common::*;
use super::error::*;
use super::mixer::Mixer;
use crate::note::{Note, Tone};

use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NoteInterval {
    pub note: Note,
    pub start_sample: Option<usize>,
    pub end_sample: Option<usize>
}

pub trait Voice: Send + Clone {
    /// Called to update a voice to match a reference voice. This should not, for example, reset
    /// envelopes or change the note that oscillators recieve.
    fn update(&mut self, reference_voice: &Self);

    fn fill_output_for_note_intervals(
        &mut self, sample_buffer: &mut [f32], note: &[NoteInterval], output_info: &OutputInfo
    );
}

struct VoiceEntry<V: Voice> {
    voice: V,
    playing_note: Option<Note>
}

pub struct VoiceSet<V: Voice, N: NoteOutputModule> {
    reference_voice: MutexPtr<V>,
    max_voices: usize,
    voice_entries: Vec<VoiceEntry<V>>,

    note_source: MutexPtr<N>,
    currently_active_notes: HashSet<Note>
}

impl<V: Voice, N: NoteOutputModule> VoiceSet<V, N> {
    /// Creates a new voice box.
    pub fn new(reference_voice: MutexPtr<V>, max_voices: usize, note_source: MutexPtr<N>) -> Self {
        debug_assert!(max_voices > 0, "Voice set with no voices");

        // Create voices
        let mut voice_entries = Vec::with_capacity(max_voices);
        {
            // Lock block
            let reference_voice_lock = reference_voice.lock().expect("Reference voice lock is poisoned");
            for _ in 0..max_voices {
                let voice = reference_voice_lock.clone();
                let voice_entry = VoiceEntry { voice, playing_note: None };
                voice_entries.push(voice_entry)
            }
        }
        let currently_active_notes = HashSet::new();

        Self { reference_voice, max_voices, voice_entries, note_source, currently_active_notes }
    }
}

impl<V: Voice, N: NoteOutputModule> SignalOutputModule for VoiceSet<V, N> {
    fn fill_output_buffer(&mut self, buffer: &mut [f32], output_info: &OutputInfo) {
        let buffer_len = buffer.len();

        let notes_per_sample = {
            self.note_source.lock().expect("Failed to lock note source").get_output(buffer_len, output_info)
        };
        debug_assert!(notes_per_sample.len() == buffer_len, "Buffer lengths do not match");

        // Get initial note intervals for notes that are on at the begining of this sample period
        let mut note_intervals = Vec::new();
        for currently_active_note in self.currently_active_notes.iter().cloned() {
            // Notes that were already active when we satrt have no start sample
            let interval = NoteInterval{ note: currently_active_note, start_sample: None, end_sample: None };
            note_intervals.push(interval);
        }

        // Gather note intervals for this sample period
        for (sample_number, note_set) in notes_per_sample.iter().enumerate() {
            let currently_active_notes_clone = self.currently_active_notes.clone();
            let newly_activated = note_set.difference(&currently_active_notes_clone);
            let newly_deactivated = currently_active_notes_clone.difference(&note_set);
            for note in newly_activated.cloned() {
                let interval = NoteInterval { note, start_sample: Some(sample_number), end_sample: None };
                note_intervals.push(interval);
                self.currently_active_notes.insert(note);
            }
            for note in newly_deactivated.cloned() {
                for existing_interval in note_intervals.iter_mut() {
                    if existing_interval.note == note && existing_interval.end_sample.is_none() {
                        existing_interval.end_sample = Some(sample_number);
                        break;
                    }
                    self.currently_active_notes.remove(&note);
                }
            }
        }

        // Setup voices to play the note intervals
        let note_intervals_by_voice = Vec::with_capacity(self.max_voices);
        for voice_entry in self.voice_entries.iter() {
            
        }

        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::oscillator::Oscillator;
    use crate::clock::SampleClock;

    use std::sync::{Arc, Mutex};
    use std::iter::FromIterator;

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
    struct TestVoice {
        osc: MutexPtr<Oscillator>,
        freq_override: MutexPtr<OscillatorFrequencyOverride>
    }

    impl TestVoice {
        fn new() -> Self {
            let mut unmutexed_osc = Oscillator::new();
            let freq_override = Arc::new(Mutex::new(OscillatorFrequencyOverride::new(Vec::new())));
            unmutexed_osc.set_frequency_override_input(freq_override.clone());
            let osc = Arc::new(Mutex::new(unmutexed_osc));
            Self { osc, freq_override }
        }
    }

    impl Voice for TestVoice {
        fn update(&mut self, reference: &Self) {
            let ref_osc = reference.osc.lock().expect("Reference Osc lock is poisoned");
            let mut osc = self.osc.lock().expect("Osc lock is poisoned");

            let ref_state = ref_osc.get_state();
            let mut state = osc.get_state_mut();
            state.pulse_width = ref_state.pulse_width;
            state.waveform = ref_state.waveform;
        }

        fn fill_output_for_note_intervals(
            &mut self, sample_buffer: &mut [f32], note_intervals: &[NoteInterval],
            output_info: &OutputInfo
        ) {
            assert!(!note_intervals.is_empty(), "Tried to get output with no note intervals");
            let buffer_len = sample_buffer.len();

            // Get freq value for each sample
            let mut freq_values = Vec::with_capacity(buffer_len);
            let mut sample_counter = 0_usize;
            for note_interval in note_intervals {
                assert!(note_interval.start_sample >= sample_counter, "Overlapping intervals");
                assert!(sample_counter < buffer_len, "Too many samples");
                while sample_counter != note_interval.start_sample {
                    freq_values.push(None);
                    sample_counter += 1;
                }

                let note_freq = note_interval.note.to_freq();
                let end_sample = note_interval.end_sample.or(Some(buffer_len)).unwrap();
                while sample_counter != end_sample {
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

    struct TestNoteSource {
        notes: HashSet<Note>,
        send_interval: Vec<bool>
    }

    impl TestNoteSource {
        fn new(notes: HashSet<Note>, n_samples: usize) -> Self {
            let send_interval = vec![true; n_samples];
            Self { notes, send_interval }
        }

        fn set_send_interval(&mut self, interval: &[bool]) {
            self.send_interval = interval.to_owned();
        }
    }

    impl NoteOutputModule for TestNoteSource {
        fn get_output(&mut self, n_samples: usize, _output_info: &OutputInfo) -> Vec<HashSet<Note>> {
            assert_eq!(n_samples, self.send_interval.len(), "What?");
            let mut ret = Vec::with_capacity(self.notes.len());
            for send in self.send_interval.iter().cloned() {
                if send {
                    ret.push(self.notes.clone());    
                }
                else {
                    ret.push(HashSet::new());
                }
            }
            ret
        }

        fn fill_output_buffer(&mut self, buffer: &mut [HashSet<Note>], output_info: &OutputInfo) {
            let output = self.get_output(buffer.len(), output_info);
            for (datum, value) in buffer.iter_mut().zip(output) {
                *datum = value;
            }
        }
    }

    fn create_test_output_info(sample_rate: usize) -> OutputInfo {
        let mut sample_clock = SampleClock::new(sample_rate);
        let sample_range = sample_clock.get_range(sample_rate);
        OutputInfo::new(sample_rate, sample_range, OutputTimestamp::empty())
    }

    #[test]
    fn get_output_with_limited_voices() {
        let notes = HashSet::from_iter([
            Note::new(1, Tone::A),
            Note::new(2, Tone::B),
            Note::new(3, Tone::C)
        ].iter().cloned());
        
        let ref_voice = TestVoice::new();
        let note_source = TestNoteSource::new(notes, 100);
        let mut voice_set = VoiceSet::new(Arc::new(Mutex::new(ref_voice)), 5, Arc::new(Mutex::new(note_source)));

        let output_info = create_test_output_info(100);
        let mut output_buffer = [0_f32; 100];
        voice_set.fill_output_buffer(&mut output_buffer, &output_info);

        assert_ne!(output_buffer[0], 0.0, "Expected some actual values");

        // TODO: Maybe we could check that the output has the frequencies we expect via DFT?
    }

    #[test]
    fn get_output_with_limited_voices_maxed_out() {
        let notes = HashSet::from_iter([
            Note::new(1, Tone::A),
            Note::new(2, Tone::B),
            Note::new(3, Tone::C)
        ].iter().cloned());

        let ref_voice = TestVoice::new();
        let note_source = TestNoteSource::new(notes, 100);
        let mut voice_set = VoiceSet::new(Arc::new(Mutex::new(ref_voice)), 1, Arc::new(Mutex::new(note_source)));

        let output_info = create_test_output_info(100);
        let mut output_buffer = [0_f32; 100];
        voice_set.fill_output_buffer(&mut output_buffer, &output_info);

        assert_ne!(output_buffer[0], 0.0, "Expected some actual values");

        // TODO: Maybe we could check that the output has the frequencies we expect via DFT?
    }

    #[test]
    fn get_output_with_unlimited_voices() {
        let notes = HashSet::from_iter([
            Note::new(1, Tone::A),
            Note::new(2, Tone::B),
            Note::new(3, Tone::C)
        ].iter().cloned());
        
        let ref_voice = TestVoice::new();
        let note_source = TestNoteSource::new(notes, 100);
        let mut voice_set = VoiceSet::new(Arc::new(Mutex::new(ref_voice)), 0, Arc::new(Mutex::new(note_source)));

        let output_info = create_test_output_info(100);
        let mut output_buffer = [0_f32; 100];
        voice_set.fill_output_buffer(&mut output_buffer, &output_info);

        assert_ne!(output_buffer[0], 0.0, "Expected some actual values");

        // TODO: Maybe we could check that the output has the frequencies we expect via DFT?
    }
}