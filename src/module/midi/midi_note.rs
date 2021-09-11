use super::super::error::{ModuleError, ModuleResult};
use super::super::common::{MutexPtr, NoteOutputModule, OutputInfo, OutputTimestamp};
use super::super::midi::MidiModuleBase;
use crate::midi;
use crate::note::Note;

use std::collections::HashSet;
use std::ops::DerefMut;

#[derive(Debug, Copy, Clone, PartialEq)]
enum NotePriority {
    Low,
    High,
    First,
    Last
}

pub struct MidiNoteOutput {
    midi_source: MutexPtr<MidiModuleBase>,
    priority: NotePriority,
    max_voices: u32,
    on_notes: Vec<u8>
}

impl MidiNoteOutput {
    pub fn new(midi_source: MutexPtr<MidiModuleBase>) -> Self {
        let priority = NotePriority::Last;
        let max_voices = 1; // Mono by default
        let on_notes = Vec::new();
        Self { midi_source, priority, max_voices, on_notes }
    }

    pub fn set_max_voices(&mut self, max_voices: u32) {
        self.max_voices = max_voices;
    }

    pub fn get_max_voices(&self) -> u32 {
        self.max_voices
    }

    // Gets all notes that are currently on
    pub fn get_notes_on_absolute(&self) -> ModuleResult<HashSet<u8>> {
        let midi_src = match self.midi_source.lock() {
            Ok(midi_src) => midi_src,
            Err(err) => {
                let msg = format!("Failed to get notes from MIDI file. Lock is poisoned!: {}", err);
                return Err(ModuleError::new(&msg));
            }
        };

        midi_src.get_notes_on_absolute()
    }
    
    /// Gets changes in note state since the last time this was called
    fn consume_notes_on_off_delta(
        &mut self, n_milliseconds: usize, timestamp: &OutputTimestamp
    ) -> ModuleResult<midi::data::NoteDelta> {
        let mut midi_src = match self.midi_source.lock() {
            Ok(midi_src) => midi_src,
            Err(err) => {
                let msg = format!("Failed to get notes from MIDI file. Lock is poisoned!: {}", err);
                return Err(ModuleError::new(&msg));
            }
        };

        midi_src.deref_mut().consume_notes_on_off_delta(n_milliseconds, timestamp)
    }

    fn get_active_notes(&self) -> Vec<u8> {
        let on_notes_len = self.on_notes.len();
        if on_notes_len as u32 <= self.max_voices {
            // If there are enough voices for all our notes then just send them all
            if self.priority == NotePriority::High || self.priority == NotePriority::Low {
                // Low and high outputs should be sorted for consistency
                let mut ret = self.on_notes.clone();
                ret.sort();
                return ret
            }
            return self.on_notes.clone();
        }

        match self.priority {
            NotePriority::High => {
                let mut on_notes_by_value = self.on_notes.clone();
                on_notes_by_value.sort();
                let split_point = on_notes_len - self.max_voices as usize;
                let (_, important_notes) = on_notes_by_value.split_at(split_point);
                important_notes.to_owned()
            },

            NotePriority::Low => {
                let mut on_notes_by_value = self.on_notes.clone();
                on_notes_by_value.sort();
                let split_point = self.max_voices as usize;
                let (important_notes, _) = on_notes_by_value.split_at(split_point);
                important_notes.to_owned()
            }
            NotePriority::First => self.on_notes.split_at(self.max_voices as usize).0.to_owned(),
            NotePriority::Last => self.on_notes.split_at(on_notes_len - self.max_voices as usize).1.to_owned()
        }
    }
}

impl NoteOutputModule for MidiNoteOutput {
    fn get_output(&mut self, n_samples: usize, output_info: &OutputInfo) -> Vec<HashSet<Note>> {
        // TODO: This does not take retriggers into account. In a normal synth if a note went off and on again
        // at the same instant the envelope would be retriggered. But that doesn't happen here...
        let n_milliseconds = n_samples * 1000 / output_info.sample_rate;
        let note_delta = match self.consume_notes_on_off_delta(n_milliseconds, &output_info.timestamp) {
            Ok(delta) => delta,
            Err(err) => {
                // TODO: Remove panic. I think that involves changing the signature of MidiMonoNoteOutput
                panic!("Failed to get MIDI notes delta: {}", err);
            }
        };

        let mut delta_iter = note_delta.iter();
        let mut next_delta = match delta_iter.next() {
            Some(cur_delta) => cur_delta,
            None => {
                // There aren't any changes for this sample period.
                // Do whatever we were doing before.
                let active_midi_notes = self.get_active_notes();
                let active_notes_len = active_midi_notes.len();
                let mut active_notes = HashSet::with_capacity(active_notes_len);
                for active_midi_note in active_midi_notes {
                    let active_note = Note::from_midi_note(active_midi_note);
                    active_notes.insert(active_note);
                }
                let mut ret = Vec::with_capacity(n_samples);
                for _ in 0..n_samples {
                    ret.push(active_notes.clone());
                }
                return ret;
            }
        };

        // calculate all the timing stuff
        let mut next_delta_start_milliseconds = next_delta.get_time_in_milliseconds(&note_delta);
        let sample_length_milliseconds = output_info.sample_rate / 1000;
        let start_milliseconds = {
            // Block in which midi_source is locked
            let midi_source_lock = match self.midi_source.lock() {
                Ok(midi_source_lock) => midi_source_lock,
                Err(err) => {
                    // TODO: Remove panic. I think that involves changing the signature of MidiMonoNoteOutput
                    panic!("MIDI source lock is poisoned!: {}", err);
                }
            };

            midi_source_lock.get_time()
        };

        let mut ret = Vec::with_capacity(n_samples);
        for i in 0..n_samples {
            // update on_notes to match current sample time
            let curr_time_milliseconds = start_milliseconds + i * sample_length_milliseconds;
            while curr_time_milliseconds >= next_delta_start_milliseconds {
                let delta_note_number = next_delta.get_note_number();
                match next_delta.get_event_type() {
                    midi::data::NoteEventType::On => self.on_notes.push(delta_note_number),
                    midi::data::NoteEventType::Off =>
                        self.on_notes.retain(|on_note_number| *on_note_number != delta_note_number)
                }

                // Get next delta if there is one
                next_delta = match delta_iter.next() {
                    Some(new_next_delta) => new_next_delta,
                    None => break // I think?
                };
                next_delta_start_milliseconds = next_delta.get_time_in_milliseconds(&note_delta);
            }

            // Insert correct notes into output vec
            let active_midi_notes = self.get_active_notes();
            let mut active_notes = HashSet::with_capacity(active_midi_notes.len());
            for active_midi_note in active_midi_notes {
                let active_note = Note::from_midi_note(active_midi_note);
                active_notes.insert(active_note);
            }

            ret.push(active_notes);
        }
        ret
    }

    fn fill_output_buffer(&mut self, buffer: &mut [HashSet<Note>], output_info: &OutputInfo) {
        // Do this all the lazy way... just calculate the whole thing and copy it over
        let output = self.get_output(buffer.len(), output_info);
        debug_assert!(output.len() == buffer.len(), "Output and initial buffer differ in size");
        for i in 0..output.len() {
            // TODO: very lazy, get rid of clone... just do a move
            buffer[i] = output[i].clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::common::OutputTimestamp;
    use crate::clock;
    use crate::util::test_util;

    use std::sync::{Arc, Mutex};

    fn get_test_midi_module() -> MidiNoteOutput {
        let path = test_util::get_test_midi_file_path();
        let midi_module_base = match MidiModuleBase::open(path) {
            Ok(midi_module_base) => midi_module_base,
            Err(err) => {
                panic!("Failed to get midi module base: {}", err);
            }
        };
        
        let arc_mutex_midi = Arc::new(Mutex::new(midi_module_base));
        MidiNoteOutput::new(arc_mutex_midi)
    }

    #[test]
    fn get_notes_delta() {
        let mut midi_module = get_test_midi_module();
        midi_module.midi_source.lock().expect("Failed to lock midi source").set_channel(Some(2));

        let delta = match midi_module.consume_notes_on_off_delta(10_000, &OutputTimestamp::empty()) {
            Ok(delta) => delta,
            Err(err) => {
                panic!("Failed to get note delta: {}", err);
            }
        };
        assert_ne!(delta.delta.len(), 0, "Expected to get notes back");

        let mut notes_on = HashSet::<u8>::new();
        for note_event in delta.delta {
            if note_event.get_event_type() == midi::data::NoteEventType::On {
                notes_on.insert(note_event.get_note_number());
            }
            else if note_event.get_event_type() == midi::data::NoteEventType::Off {
                notes_on.remove(&note_event.get_note_number());
            }
        }
        assert_eq!(notes_on.len(), 0, "Expected every note that was on to have an off counterpart");
    }

    #[test]
    fn get_notes_on_absolute() {
        let midi_module = get_test_midi_module();

        let target_milliseconds = 8854; // Just trust me bro. It'll have one note on
        midi_module.midi_source.lock().expect("Failed to lock midi source").set_time(target_milliseconds);
        
        let notes_on = match midi_module.get_notes_on_absolute() {
            Ok(notes_on) => notes_on,
            Err(err) => {
                panic!("Failed to get notes on: {}", err);
            }
        };

        assert_eq!(notes_on.len(), 2, "Expected there to be two notes on");
        assert!(notes_on.contains(&36) && notes_on.contains(&73), "Expected notes 36 & 73 to be on");
    }

    #[test]
    fn get_active_notes() {
        const ON_NOTES: &[u8] = &[1, 4, 3, 5, 2];
        let mut midi_module = get_test_midi_module();
        midi_module.on_notes = ON_NOTES.to_owned();
        
        midi_module.priority = NotePriority::First;
        let mut active_notes = midi_module.get_active_notes();
        assert_eq!(*active_notes, [1]);

        midi_module.priority = NotePriority::Last;
        active_notes = midi_module.get_active_notes();
        assert_eq!(*active_notes, [2]);

        midi_module.priority = NotePriority::Low;
        active_notes = midi_module.get_active_notes();
        assert_eq!(*active_notes, [1]);

        midi_module.priority = NotePriority::High;
        active_notes = midi_module.get_active_notes();
        assert_eq!(*active_notes, [5]);

        midi_module.max_voices = 3;

        midi_module.priority = NotePriority::Low;
        active_notes = midi_module.get_active_notes();
        assert_eq!(*active_notes, [1, 2, 3]);

        midi_module.priority = NotePriority::High;
        active_notes = midi_module.get_active_notes();
        assert_eq!(*active_notes, [3, 4, 5]);

        midi_module.priority = NotePriority::First;
        active_notes = midi_module.get_active_notes();
        assert_eq!(*active_notes, [1, 4, 3]);

        midi_module.priority = NotePriority::Last;
        active_notes = midi_module.get_active_notes();
        assert_eq!(*active_notes, [3, 5, 2]);

        midi_module.max_voices = 10;

        midi_module.priority = NotePriority::Low;
        active_notes = midi_module.get_active_notes();
        assert_eq!(*active_notes, [1, 2, 3, 4, 5]);

        midi_module.priority = NotePriority::High;
        active_notes = midi_module.get_active_notes();
        assert_eq!(*active_notes, [1, 2, 3, 4, 5]);

        midi_module.priority = NotePriority::First;
        active_notes = midi_module.get_active_notes();
        assert_eq!(*active_notes, [1, 4, 3, 5, 2]);

        midi_module.priority = NotePriority::Last;
        active_notes = midi_module.get_active_notes();
        assert_eq!(*active_notes, [1, 4, 3, 5, 2]);
    }

    #[test]
    fn get_mono_output() {
        const SAMPLE_RATE: usize = 10_000;
        const N_SAMPLES: usize = SAMPLE_RATE * 10; // 10 seconds
        let mut midi_module = get_test_midi_module();

        let mut sample_clock = clock::SampleClock::new(SAMPLE_RATE);
        let sample_range = sample_clock.get_range(N_SAMPLES);
        let output_info = OutputInfo::new_basic(SAMPLE_RATE, sample_range);

        let output = midi_module.get_output(N_SAMPLES, &output_info);
        assert_eq!(output.len(), N_SAMPLES, "Output length does not match expected");
        // Theres not really a great way I can think of to test this...
    }
}