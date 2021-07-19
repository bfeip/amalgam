pub mod midi_note;

use crate::midi;
use super::error::*;
use super::common::*;

use std::collections::HashSet;

struct TimestampDuration {
    start_milliseconds: usize,
    end_milliseconds: usize
}

pub struct MidiModuleBase {
    data: midi::data::MidiData,
    track: usize,
    channel: Option<usize>,

    playing: bool,

    cache_timestamp: OutputTimestamp,
    cache_timestamp_duration: TimestampDuration,
    cached_note_delta: Option<midi::data::NoteDelta>,

    milliseconds_read: usize,
}

impl MidiModuleBase {
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> ModuleResult<Self> {
        let data = match midi::data::MidiData::from_file(path) {
            Ok(data) => data,
            Err(err) => {
                let msg = format!("Failed to create MIDI file module: {}", err);
                return Err(ModuleError::new(&msg));
            }
        };
        let track = 0;
        let channel = None;

        let playing = true;

        let cache_timestamp = OutputTimestamp::empty();
        let cache_timestamp_duration = TimestampDuration{ start_milliseconds: 0, end_milliseconds: 0 };
        let cached_note_delta = None;

        let milliseconds_read = 0;

        Ok(Self {
            data,
            track,
            channel,
            playing,
            cache_timestamp,
            cache_timestamp_duration,
            cached_note_delta,
            milliseconds_read })
    }

    pub fn set_track(&mut self, track_number: usize) -> ModuleResult<()> {
        let track_len = self.data.get_tracks().len();
        if self.data.get_tracks().len() <= track_number {
            let msg = format!(
                "MIDI track out of range. Attempted to set to {}, max track: {}", track_number, track_len
            );
            return Err(ModuleError::new(&msg));
        }
        self.track = track_number;
        Ok(())
    }

    pub fn get_track(&self) -> usize {
        self.track
    }

    pub fn set_channel(&mut self, channel: Option<usize>) {
        self.channel = channel
    }

    pub fn get_channel(&self) -> Option<usize> {
        self.channel
    }

    pub fn set_time(&mut self, milliseconds: usize) {
        self.milliseconds_read = milliseconds;
        self.invalidate_cache();
    }

    pub fn rewind_time(&mut self, milliseconds: usize) {
        self.milliseconds_read = self.milliseconds_read.saturating_sub(milliseconds);
        self.invalidate_cache();
    }

    pub fn fastforward_time(&mut self, milliseconds: usize) {
        self.milliseconds_read += milliseconds;
        self.invalidate_cache();
    }

    pub fn get_time(&self) -> usize {
        self.milliseconds_read
    }

    fn invalidate_cache(&mut self) {
        self.cached_note_delta = None;
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

    pub fn consume_notes_on_off_delta(
        &mut self, n_milliseconds: usize, timestamp: &OutputTimestamp
    ) -> ModuleResult<midi::data::NoteDelta> {
        if *timestamp == self.cache_timestamp {
            // We're getting a delta again for the sample range we consumed last time
            match &self.cached_note_delta {
                Some(cached_note_delta) => {
                    // We already got a note delta for this sample range, return it
                    return Ok(cached_note_delta.clone());
                }
                None => {
                    // We have a re-read the past sample range
                    let duration = &self.cache_timestamp_duration;
                    debug_assert!(
                        duration.end_milliseconds - duration.start_milliseconds == n_milliseconds,
                        "Duration we're re-reading now does not match duration we read last time"
                    );
                    self.milliseconds_read = duration.start_milliseconds;
                }
            }
        }

        let start_milliseconds = self.milliseconds_read;
        let end_milliseconds = start_milliseconds + n_milliseconds;

        let note_delta_result = self.data.get_notes_delta(self.track, self.channel, start_milliseconds, end_milliseconds);
        match note_delta_result {
            Ok(notes_delta) => {
                self.cached_note_delta = Some(notes_delta.clone());
                self.cache_timestamp = timestamp.clone();
                self.cache_timestamp_duration = TimestampDuration { start_milliseconds, end_milliseconds };
                self.milliseconds_read += n_milliseconds;

                Ok(notes_delta)
            },
            Err(err) => {
                let msg = format!("Failed to get notes delta from MIDI: {}", err);
                return Err(ModuleError::new(&msg));
            }
        }
    }
}