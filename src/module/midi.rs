mod midi_mono_note;

use crate::midi;
use super::error::*;

use midi_mono_note::MidiMonoNoteOutput;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

pub struct MidiFile<'data> {
    data: midi::data::MidiData<'data>,
    track: usize,
    channel: usize,
    midi_ticks_read: usize
}

impl<'data> MidiFile<'data> {
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> ModuleResult<Self> {
        let data = match midi::data::MidiData::from_file(path) {
            Ok(data) => data,
            Err(err) => {
                let msg = format!("Failed to create MIDI file module: {}", err);
                return Err(ModuleError::new(&msg));
            }
        };
        let track = 0;
        let channel = 0;
        let midi_ticks_read = 0;

        Ok(Self { data, track, channel, midi_ticks_read })
    }

    pub fn set_channel(&mut self, channel: usize) {
        self.channel = channel
    }

    pub fn get_channel(&self) -> usize {
        self.channel
    }

    pub fn get_notes_on_absolute(&self) -> ModuleResult<HashSet<u8>> {
        let notes_on_result = self.data.get_notes_on_absolute(self.track, self.channel, self.midi_ticks_read);
        match notes_on_result {
            Ok(notes_on) => Ok(notes_on),
            Err(err) => {
                let msg = format!("Failed to get notes on from MIDI: {}", err);
                return Err(ModuleError::new(&msg));
            }
        }
    }

    pub fn get_mono_note_output(midi_src_mutex_ptr: Arc<Mutex<Self>>) -> MidiMonoNoteOutput<'data> {
        MidiMonoNoteOutput::new(midi_src_mutex_ptr)
    }
}