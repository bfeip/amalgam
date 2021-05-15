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
    pub fn get_note_on_absolute(&self) -> ModuleResult<HashSet<u8>> {
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