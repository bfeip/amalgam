use super::common::*;
use crate::note::Note;

use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NoteInterval {
    pub note: Note,
    pub start_sample: Option<usize>,
    pub end_sample: Option<usize>
}

impl NoteInterval {
    const fn new(note: Note, start_sample: Option<usize>, end_sample: Option<usize>) -> Self {
        Self { note, start_sample, end_sample }
    }

    fn overlaps(&self, other: &NoteInterval) -> bool {
        let this_start = self.start_sample.or(Some(0)).unwrap();
        let this_end = self.end_sample.or(Some(usize::MAX)).unwrap();
        let other_start = other.start_sample.or(Some(0)).unwrap();
        let other_end = other.end_sample.or(Some(usize::MAX)).unwrap();

        if this_start >= other_start && this_start < other_end {
            // This starts within other
            return true;
        } 
        if this_end > other_start && this_end <= other_end {
            // This ends within other
            return true;
        }
        if other_start > this_start && other_start < this_end {
            // Other must be fully contained within this
            return true;
        }

        return false;
    }
}

pub trait Voice: Send + Clone {
    /// Called to update a voice to match a reference voice. This should not, for example, reset
    /// envelopes or change the note that oscillators recieve.
    fn update(&mut self, reference_voice: &Self);

    fn fill_output_for_note_intervals(
        &mut self, sample_buffer: &mut [f32], intervals: &[NoteInterval], output_info: &OutputInfo
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
            // Notes that were already active when we start have no start sample
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

        // Setup voices to play the note intervals that were continuing to play from last sample period
        let mut note_intervals_by_voice = Vec::with_capacity(self.max_voices);
        for (i, voice_entry) in self.voice_entries.iter().enumerate() {
            let voice_note_intervals = Vec::<NoteInterval>::new();
            note_intervals_by_voice.push(voice_note_intervals);

            if let Some(playing_note) = voice_entry.playing_note {
                // This voice was playing a note when the sample started. Find what's going on with that note now
                // and set it to be played by this voice.
                let mut found_interval = false;
                for note_interval in note_intervals.iter() {
                    if note_interval.note == playing_note && note_interval.start_sample.is_none() {
                        // We found our interval
                        note_intervals_by_voice[i].push(note_interval.clone());
                        found_interval = true;
                        break;
                    }
                }
                debug_assert!(found_interval, "Failed to find playing note's continuing interval");
            }
        }

        // Group remaining note intervals by voice such that they are not overlapping
        for note_interval in note_intervals {
            if note_interval.start_sample.is_none() {
                // Intervals that were already playing at the begining of the sample period should have been set
                // up by the previous code block
                continue;
            }
            for voice_index in 0..self.max_voices {
                let voice_intervals = &mut note_intervals_by_voice[voice_index];
                let mut overlap = false;
                for existing_interval in voice_intervals.iter() {
                    if note_interval.overlaps(existing_interval) {
                        overlap = true;
                        break;
                    }
                }
                if !overlap {
                    voice_intervals.push(note_interval.clone());
                }
            }
        }

        // Send intervals to voices and get output
        buffer.fill(0_f32);
        for (i, voice_entry) in self.voice_entries.iter_mut().enumerate() {
            let mut voice_output = vec![0_f32; buffer_len];
            let intervals = &note_intervals_by_voice[i];
            voice_entry.voice.fill_output_for_note_intervals(&mut voice_output, intervals, output_info);
            for i in 0..voice_output.len() {
                buffer[i] += voice_output[i];
            }
        }
        compress_audio(buffer, CompressionMode::Compress);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::oscillator::Oscillator;
    use crate::note::Tone;
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
    fn note_interval_overlap() {
        const NOTE: Note = Note::new(4, Tone::C);
        const PERMA_HELD: NoteInterval = NoteInterval::new(NOTE, None, None);
        const HELD_START: NoteInterval = NoteInterval::new(NOTE, None, Some(100));
        const HELD_END: NoteInterval = NoteInterval::new(NOTE, Some(100), None);
        const ONE_TO_HUNDRED: NoteInterval = NoteInterval::new(NOTE, Some(1), Some(100));
        const TEN_TO_NINETY: NoteInterval = NoteInterval::new(NOTE, Some(10), Some(90));
        const ONE_TO_FIFTY: NoteInterval = NoteInterval::new(NOTE, Some(1), Some(50));
        assert!(PERMA_HELD.overlaps(&PERMA_HELD));
        assert!(PERMA_HELD.overlaps(&HELD_START));
        assert!(PERMA_HELD.overlaps(&HELD_END));
        assert!(PERMA_HELD.overlaps(&ONE_TO_FIFTY));

        assert!(HELD_START.overlaps(&PERMA_HELD));
        assert!(HELD_START.overlaps(&HELD_START));
        assert!(HELD_START.overlaps(&ONE_TO_HUNDRED));
        assert!(HELD_START.overlaps(&TEN_TO_NINETY));
        assert!(!HELD_START.overlaps(&HELD_END));

        assert!(HELD_END.overlaps(&PERMA_HELD));
        assert!(!HELD_END.overlaps(&HELD_START));
        assert!(HELD_END.overlaps(&HELD_END));

        assert!(ONE_TO_HUNDRED.overlaps(&PERMA_HELD));
        assert!(ONE_TO_HUNDRED.overlaps(&HELD_START));
        assert!(!ONE_TO_HUNDRED.overlaps(&HELD_END));
        assert!(ONE_TO_HUNDRED.overlaps(&ONE_TO_HUNDRED));
        assert!(ONE_TO_HUNDRED.overlaps(&TEN_TO_NINETY));
        assert!(ONE_TO_HUNDRED.overlaps(&ONE_TO_FIFTY));

        assert!(TEN_TO_NINETY.overlaps(&ONE_TO_FIFTY));
        assert!(TEN_TO_NINETY.overlaps(&ONE_TO_HUNDRED));
    }
}