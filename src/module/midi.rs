pub mod midi_note;

use crate::midi;
use crate::midi::data::NoteDelta;
use crate::{SynthError, SynthResult};

use std::collections::HashSet;
use std::time::Instant;
use std::cell::{Cell, RefCell};

#[derive(Debug, Clone, Copy, Hash)]
struct TimestampDuration {
    start_microseconds: usize,
    end_microseconds: usize
}

#[derive(Debug, Clone)]
struct MidiCache {
    timestamp: Instant,
    timestamp_duration: TimestampDuration,
    cached_note_delta: Option<NoteDelta>,
}

impl MidiCache {
    fn new() -> Self {
        let timestamp = Instant::now();
        let timestamp_duration = TimestampDuration{ start_microseconds: 0, end_microseconds: 0 };
        let cached_note_delta = None;
        Self {
            timestamp,
            timestamp_duration,
            cached_note_delta
        }
    }

    fn invalidate(&mut self) {
        self.cached_note_delta = None;
    }

    fn set_note_delta(&mut self, time: &Instant, duration: &TimestampDuration, delta: &NoteDelta) {
        if *time != self.timestamp {
            self.invalidate();
            self.timestamp = *time;
            self.timestamp_duration = *duration;
        }
        self.cached_note_delta = Some(delta.clone());
    }

    fn try_get_note_delta(&self, time: &Instant) -> Option<&NoteDelta> {
        if *time == self.timestamp {
            return self.cached_note_delta.as_ref();
        }
        None
    }
}

pub struct MidiModuleBase {
    data: midi::data::MidiData,
    track: usize,
    channel: Option<usize>,
    playing: bool,
    cache: RefCell<MidiCache>,
    microseconds_read: Cell<usize>,
}

impl MidiModuleBase {
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> SynthResult<Self> {
        let data = match midi::data::MidiData::from_file(path) {
            Ok(data) => data,
            Err(err) => {
                let msg = format!("Failed to create MIDI file module: {}", err);
                return Err(SynthError::new(&msg));
            }
        };
        let track = 0;
        let channel = None;
        let playing = true;
        let cache = RefCell::new(MidiCache::new());
        let microseconds_read = Cell::new(0);

        Ok(Self {
            data,
            track,
            channel,
            playing,
            cache,
            microseconds_read 
        })
    }

    pub fn set_track(&mut self, track_number: usize) -> SynthResult<()> {
        let track_len = self.data.get_tracks().len();
        if self.data.get_tracks().len() <= track_number {
            let msg = format!(
                "MIDI track out of range. Attempted to set to {}, max track: {}", track_number, track_len
            );
            return Err(SynthError::new(&msg));
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

    pub fn set_time(&mut self, microseconds: usize) {
        self.microseconds_read = Cell::new(microseconds);
        self.invalidate_cache();
    }

    pub fn rewind_time(&mut self, microseconds: usize) {
        let new_time = self.microseconds_read.get().saturating_sub(microseconds);
        self.microseconds_read.set(new_time);
        self.invalidate_cache();
    }

    pub fn fastforward_time(&mut self, microseconds: usize) {
        let new_time = self.microseconds_read.get() + microseconds;
        self.microseconds_read.set(new_time);
        self.invalidate_cache();
    }

    pub fn get_time(&self) -> usize {
        self.microseconds_read.get()
    }

    fn invalidate_cache(&mut self) {
        self.cache.get_mut().invalidate();
    }

    pub fn get_notes_on_absolute(&self) -> SynthResult<HashSet<u8>> {
        let notes_on_result = self.data.get_notes_on_absolute(self.track, self.channel, self.microseconds_read.get());
        match notes_on_result {
            Ok(notes_on) => Ok(notes_on),
            Err(err) => {
                let msg = format!("Failed to get notes on from MIDI: {}", err);
                return Err(SynthError::new(&msg));
            }
        }
    }

    pub fn read_notes_on_off_delta(
        &self, n_microseconds: usize, timestamp: &Instant
    ) -> SynthResult<NoteDelta> {
        let mut cache = self.cache.borrow_mut();
        if let Some(cached_deltas) = cache.try_get_note_delta(timestamp) {
            // We already got these deltas earlier, just send them again
            return Ok(cached_deltas.clone());
        }

        let start_microseconds = self.microseconds_read.get();
        let end_microseconds = start_microseconds + n_microseconds;

        let note_delta_result = self.data.get_notes_delta(
            self.track,
            self.channel,
            start_microseconds,
            end_microseconds
        );
        match note_delta_result {
            Ok(notes_delta) => {
                let duration = TimestampDuration { start_microseconds, end_microseconds };
                cache.set_note_delta(timestamp, &duration, &notes_delta);
                self.microseconds_read.set(start_microseconds + n_microseconds);

                Ok(notes_delta)
            },
            Err(err) => {
                let msg = format!("Failed to get notes delta from MIDI: {}", err);
                return Err(SynthError::new(&msg));
            }
        }
    }
}