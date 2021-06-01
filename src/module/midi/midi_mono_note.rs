use super::super::error::{ModuleError, ModuleResult};
use super::super::common::{MutexPtr, NoteOutputModule, OutputInfo};
use super::super::midi::MidiModuleBase;
use crate::midi;
use crate::note::Note;

use std::collections::HashSet;
use std::ops::DerefMut;

#[derive(Debug, Copy, Clone, PartialEq)]
enum MidiMonoNotePriority {
    Low,
    High,
    First,
    Last
}

pub struct MidiMonoNoteOutput {
    midi_source: MutexPtr<MidiModuleBase>,
    priority: MidiMonoNotePriority,
    on_notes: Vec<u8>
}

impl MidiMonoNoteOutput {
    pub fn new(midi_source: MutexPtr<MidiModuleBase>) -> Self {
        let priority = MidiMonoNotePriority::Last;
        let on_notes = Vec::new();
        Self { midi_source, priority, on_notes }
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
    fn consume_notes_on_off_delta(&mut self, n_milliseconds: usize) -> ModuleResult<midi::data::NoteDelta> {
        let mut midi_src = match self.midi_source.lock() {
            Ok(midi_src) => midi_src,
            Err(err) => {
                let msg = format!("Failed to get notes from MIDI file. Lock is poisoned!: {}", err);
                return Err(ModuleError::new(&msg));
            }
        };

        midi_src.deref_mut().consume_notes_on_off_delta(n_milliseconds)
    }

    fn get_currently_on(&self) -> Option<u8> {
        match self.priority {
            MidiMonoNotePriority::High => {
                let mut max_note = Option::<u8>::None;
                for note in self.on_notes.iter().cloned() {
                    if max_note.is_none() || note > max_note.unwrap() {
                        max_note = Some(note);
                    }
                }
                max_note
            },
            MidiMonoNotePriority::Low => {
                let mut min_note = Option::<u8>::None;
                for note in self.on_notes.iter().cloned() {
                    if min_note.is_none() || note < min_note.unwrap() {
                        min_note = Some(note);
                    }
                }
                min_note
            }
            MidiMonoNotePriority::First => self.on_notes.first().cloned(),
            MidiMonoNotePriority::Last => self.on_notes.last().cloned()
        }
    }
}

impl NoteOutputModule for MidiMonoNoteOutput {
    fn get_output(&mut self, n_samples: usize, output_info: &OutputInfo) -> Vec<Option<Note>> {
        let n_milliseconds = n_samples * 1000 / output_info.sample_rate;
        let note_delta = match self.consume_notes_on_off_delta(n_milliseconds) {
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
                match self.get_currently_on() {
                    Some(current_midi_note) => {
                        // There's no changes and we already have a note active from a previous sample period.
                        // Just play it.
                        let current_note = Note::from_midi_note(current_midi_note);
                        return vec![Some(current_note); n_samples]
                    },
                    None => {
                        // Nothing is happening... Report back as such
                        return vec![None; n_samples];
                    }
                }
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
                next_delta = match delta_iter.next() {
                    Some(new_next_delta) => new_next_delta,
                    None => next_delta
                };
                next_delta_start_milliseconds = next_delta.get_time_in_milliseconds(&note_delta);
            }
            // Insert correct note (if there is one) into output vec
            let midi_note = self.get_currently_on();
            let actual_note = match midi_note {
                Some(midi_note) => Some(Note::from_midi_note(midi_note)),
                None => None
            };
            ret.push(actual_note);
        }
        ret
    }

    fn fill_output_buffer(&mut self, buffer: &mut [Option<Note>], output_info: &OutputInfo) {
        // Do this all the lazy way... just calculate the whole thing and copy it over
        let output = self.get_output(buffer.len(), output_info);
        debug_assert!(output.len() == buffer.len(), "Output and initial buffer differ in size");
        for i in 0..output.len() {
            buffer[i] = output[i];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_util;

    use std::sync::{Arc, Mutex};

    fn get_test_midi_module() -> MidiMonoNoteOutput {
        let path = test_util::get_test_midi_file_path();
        let midi_module_base = match MidiModuleBase::open(path) {
            Ok(midi_module_base) => midi_module_base,
            Err(err) => {
                panic!("Failed to get midi module base: {}", err);
            }
        };
        
        let arc_mutex_midi = Arc::new(Mutex::new(midi_module_base));
        MidiMonoNoteOutput::new(arc_mutex_midi)
    }

    #[test]
    fn get_notes_delta() {
        let mut midi_module = get_test_midi_module();
        midi_module.midi_source.lock().expect("Failed to lock midi source").set_channel(Some(2));

        let delta = match midi_module.consume_notes_on_off_delta(10_000) {
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
}