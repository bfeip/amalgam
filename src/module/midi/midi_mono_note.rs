use super::MidiModuleBase;
use super::super::error::{ModuleError, ModuleResult};
use crate::midi;

use std::{ops::DerefMut, sync::{Mutex, Arc}};
use std::collections::HashSet;

type MidiFileMutexPtr = Arc<Mutex<MidiModuleBase>>;

#[derive(Debug, Copy, Clone, PartialEq)]
enum MidiMonoNotePriority {
    Low,
    High,
    First,
    Last
}

pub struct MidiMonoNoteOutput {
    midi_source: MidiFileMutexPtr,
    priority: MidiMonoNotePriority,
    on_notes: Vec<u8>
}

impl MidiMonoNoteOutput {
    pub fn new(midi_source: MidiFileMutexPtr) -> Self {
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
    
    // Gets changes in note state since the last time this was called
    pub fn get_notes_on_off_delta(&mut self, n_milliseconds: usize) -> ModuleResult<midi::data::NoteDelta> {
        let mut midi_src = match self.midi_source.lock() {
            Ok(midi_src) => midi_src,
            Err(err) => {
                let msg = format!("Failed to get notes from MIDI file. Lock is poisoned!: {}", err);
                return Err(ModuleError::new(&msg));
            }
        };

        midi_src.deref_mut().get_notes_on_off_delta(n_milliseconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_util;

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

        let delta = match midi_module.get_notes_on_off_delta(10_000) {
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