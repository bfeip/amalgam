use super::common::*;
use crate::note::{Note, NoteInterval};
use crate::SignalLogger;

use std::collections::HashSet;

pub trait Voice: Send + Clone {
    /// Called to update a voice to match a reference voice. This should not, for example, reset
    /// envelopes or change the note that oscillators receive.
    fn update(&mut self, reference_voice: &Self);

    fn fill_output_for_note_intervals(
        &mut self,
        sample_buffer: &mut [f32],
        intervals: &[NoteInterval],
        output_info: &OutputInfo,
        voice_number: usize
    );
}

struct VoiceEntry<V: Voice> {
    voice: V,
    playing_note: Option<Note>
}

pub struct VoiceSet<V, N>
where 
    V: Voice + ?Sized,
    N: NoteOutputModule + ?Sized
{
    reference_voice: Connectable<V>,
    max_voices: usize,
    voice_entries: Vec<VoiceEntry<V>>,
    next_voice_index: usize, // Used to round robin though voices while playing notes

    note_source: Connectable<N>,
    currently_active_notes: HashSet<Note>,
    signal_logger: SignalLogger,
}

impl<V, N> VoiceSet<V, N>
where 
    V: Voice + ?Sized,
    N: NoteOutputModule + ?Sized
{
    /// Creates a new voice box.
    pub fn new(reference_voice: Connectable<V>, max_voices: usize, note_source: Connectable<N>) -> Self {
        debug_assert!(max_voices > 0, "Voice set with no voices");

        // Create voices
        let mut voice_entries = Vec::with_capacity(max_voices);
        {
            // Lock block
            let reference_voice_lock = reference_voice.lock();
            for _ in 0..max_voices {
                let voice = reference_voice_lock.clone();
                let voice_entry = VoiceEntry { voice, playing_note: None };
                voice_entries.push(voice_entry)
            }
        }
        let next_voice_index = 0;
        let currently_active_notes = HashSet::new();

        #[cfg(feature = "signal_logging")]
        let signal_logger = SignalLogger::new("voices_signal.txt");
        #[cfg(not(feature = "signal_logging"))]
        let signal_logger = SignalLogger::new_sink();

        Self {
            reference_voice,
            max_voices,
            voice_entries,
            next_voice_index,
            note_source,
            currently_active_notes,
            signal_logger
        }
    }
}

impl<V, N> SignalOutputModule for VoiceSet<V, N>
where 
    V: Voice + ?Sized,
    N: NoteOutputModule + ?Sized
{
    fn fill_output_buffer(&mut self, buffer: &mut [f32], output_info: &OutputInfo) {
        let buffer_len = buffer.len();

        let note_intervals = {
            self.note_source.lock().get_output(buffer_len, output_info)
        };

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
                    if note_interval.note == playing_note && note_interval.start.is_none() {
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
            if note_interval.start.is_none() {
                // Intervals that were already playing at the beginning of the sample period should have been set
                // up by the previous code block (or _a_ previous code block, if this comment is outdated).
                continue;
            }
            for _ in 0..self.max_voices {
                // Cycle through all voices looking for one to play this interval
                let voice_intervals = &mut note_intervals_by_voice[self.next_voice_index];
                let mut overlap = false;
                for existing_interval in voice_intervals.iter() {
                    if note_interval.overlaps(existing_interval) {
                        overlap = true;
                        break;
                    }
                }

                self.next_voice_index = (self.next_voice_index + 1) % self.max_voices;
                if !overlap {
                    voice_intervals.push(note_interval.clone());
                    break;
                }
            }
        }

        // Send intervals to voices and get output
        buffer.fill(0_f32);
        for (voice_number, voice_entry) in self.voice_entries.iter_mut().enumerate() {
            let intervals = &note_intervals_by_voice[voice_number];

            let mut voice_output = vec![0_f32; buffer_len];
            voice_entry.voice.fill_output_for_note_intervals(&mut voice_output, intervals, output_info, voice_number);
            for i in 0..voice_output.len() {
                buffer[i] += voice_output[i];
            }

            let source_str = format!("Voice_{}", voice_number);
            if let Err(err) = self.signal_logger.log(source_str, &voice_output) {
                panic!("Failed to write voice logs: {}", err);
            }

            // Check if this is playing a note till the end of the sample. If it is make sure to record
            // that so we can continue playing it correctly in the next sample period
            // TODO: Pretty sure note intervals are guaranteed to be in order and thus we don't need to
            // iterate them like this. But I'm just being safe and I'm too lazy to check right now.
            let mut currently_playing = None;
            for interval in intervals {
                if interval.end.is_none() {
                    currently_playing = Some(interval.note);
                    break;
                }
            }
            voice_entry.playing_note = currently_playing;
            
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

    use core::panic;
    use std::iter::FromIterator;

    struct OscillatorFrequencyOverride {
        freqs: Vec<Option<f32>>
    }

    impl OscillatorFrequencyOverride {
        fn new() -> Self {
            Self { freqs: Vec::new() }
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
        voice_number: Option<usize>,
        osc: Connectable<Oscillator>,
        freq_override: Connectable<OscillatorFrequencyOverride>
    }

    impl TestVoice {
        fn new() -> Self {
            let voice_number = None;
            let mut unconnected_osc = Oscillator::new();
            let freq_override: Connectable<OscillatorFrequencyOverride> = OscillatorFrequencyOverride::new().into();
            unconnected_osc.set_frequency_override_input(freq_override.clone().into());
            let osc = unconnected_osc.into();
            Self { voice_number, osc, freq_override }
        }
    }

    impl Voice for TestVoice {
        fn update(&mut self, reference: &Self) {
            let ref_osc = reference.osc.lock();
            let mut osc = self.osc.lock();

            osc.set_pulse_width(ref_osc.get_pulse_width());
            osc.set_waveform(ref_osc.get_waveform());
        }

        fn fill_output_for_note_intervals(
            &mut self, sample_buffer: &mut [f32], note_intervals: &[NoteInterval],
            output_info: &OutputInfo, voice_number: usize
        ) {
            assert!(!note_intervals.is_empty(), "Tried to get output with no note intervals");
            let buffer_len = sample_buffer.len();

            // Check that voice number is consistent
            if *self.voice_number.get_or_insert(voice_number) != voice_number {
                panic!("Expected voice number the remain consistent");
            }

            // Get freq value for each sample
            let mut freq_values = Vec::with_capacity(buffer_len);
            let mut sample_counter = 0_usize;
            for note_interval in note_intervals {
                // Some checks that note intervals appear in the expected order and do not overlap one another
                let is_correct_start_note = note_interval.start.is_none() && sample_counter == 0;
                let is_ordered = note_interval.start.unwrap_or(sample_counter) >= sample_counter;
                assert!(is_correct_start_note || is_ordered, "Overlapping intervals");
                assert!(sample_counter < buffer_len, "Too many samples");

                // If start sample is None that means the note started in a previous sample period and we enter this
                // sample period with the note already playing
                if note_interval.start.is_some() {
                    while sample_counter != note_interval.start.unwrap() {
                        // Until the next note is played just push nothing
                        freq_values.push(None);
                        sample_counter += 1;
                    }
                }

                let note_freq = note_interval.note.to_freq();
                let end_sample = note_interval.end.unwrap_or(buffer_len);
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
                let mut freq_override = self.freq_override.lock();
                freq_override.set(freq_values.clone());
            }

            self.osc.lock().fill_output_buffer(sample_buffer, output_info);
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
    }

    impl TestNoteSource {
        fn new(notes: HashSet<Note>) -> Self {
            Self { notes }
        }
    }

    impl NoteOutputModule for TestNoteSource {
        fn get_output(&mut self, n_samples: usize, _output_info: &OutputInfo) -> Vec<NoteInterval> {
            let mut intervals = Vec::with_capacity(self.notes.len());
            for note in self.notes.iter().cloned() {
                let interval = NoteInterval::new(note, Some(0), Some(n_samples));
                intervals.push(interval);
            }
            intervals
        }
    }

    fn create_test_output_info(sample_rate: usize) -> OutputInfo {
        let mut sample_clock = SampleClock::new(sample_rate);
        let sample_range = sample_clock.get_range(sample_rate);
        OutputInfo::new_basic(sample_rate, sample_range)
    }

    #[test]
    fn get_output_with_limited_voices() {
        let notes = HashSet::from_iter([
            Note::new(1, Tone::A),
            Note::new(2, Tone::B),
            Note::new(3, Tone::C)
        ].iter().cloned());
        
        let ref_voice = TestVoice::new();
        let note_source = TestNoteSource::new(notes);
        let mut voice_set = VoiceSet::new(ref_voice.into(), 5, Connectable::<TestNoteSource>::from(note_source));

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
        let note_source = TestNoteSource::new(notes);
        let mut voice_set = VoiceSet::new(ref_voice.into(), 1, Connectable::<TestNoteSource>::from(note_source));

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