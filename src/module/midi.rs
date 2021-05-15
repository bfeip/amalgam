mod midi_mono_note;

use crate::midi;
use super::error::*;
use super::traits::*;

use midi_mono_note::MidiMonoNoteOutput;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

pub struct MidiModuleBase<'data> {
    data: midi::data::MidiData<'data>,
    track: usize,
    channel: usize,

    playing: bool,

    cache_timestamp: OutputTimestamp,
    cached_note_delta: Vec<u8>,

    milliseconds_read: usize,
}

impl<'data> MidiModuleBase<'data> {
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

        let playing = true;

        let cache_timestamp = OutputTimestamp::empty();
        let cached_note_delta = Vec::new();

        let milliseconds_read = 0;

        Ok(Self {
            data,
            track,
            channel,
            playing,
            cache_timestamp,
            cached_note_delta,
            milliseconds_read })
    }

    pub fn set_channel(&mut self, channel: usize) {
        self.channel = channel
    }

    pub fn get_channel(&self) -> usize {
        self.channel
    }

    pub fn get_notes_on_absolute(&self) -> ModuleResult<HashSet<u8>> {
        let notes_on_result = self.data.get_notes_on_absolute(self.track, self.channel, self.milliseconds_read);
        match notes_on_result {
            Ok(notes_on) => Ok(notes_on),
            Err(err) => {
                let msg = format!("Failed to get notes on from MIDI: {}", err);
                return Err(ModuleError::new(&msg));
            }
        }
    }

    pub fn get_notes_on_off_delta(&mut self, n_milliseconds: usize) -> ModuleResult<midi::data::NoteDelta> {
        let start_time = self.milliseconds_read;
        let end_time = start_time + n_milliseconds;
        let note_delta_result = self.data.get_notes_delta(self.track, self.channel, start_time, end_time);
        match note_delta_result {
            Ok(notes_delta) => Ok(notes_delta),
            Err(err) => {
                let msg = format!("Failed to get notes delta from MIDI: {}", err);
                return Err(ModuleError::new(&msg));
            }
        }
    }

    pub fn get_mono_note_output(midi_src_mutex_ptr: Arc<Mutex<Self>>) -> MidiMonoNoteOutput<'data> {
        MidiMonoNoteOutput::new(midi_src_mutex_ptr)
    }
}