use super::parser;
use super::error::*;

use std::collections::HashSet;

const MICROSECONDS_PER_MINUTE: u32 = 60_000_000;
const DEFAULT_TEMPO: u32 = MICROSECONDS_PER_MINUTE / 120;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidiData {
    tracks: Vec<Track>,
    time_division: parser::TimeDivision
}

impl MidiData {
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> MidiResult<Self> {
        let parser_data = match parser::MidiData::from_file(path) {
            Ok(parser_data) => parser_data,
            Err(err) => {
                let msg = format!("Failed to parse MIDI file: {}", err);
                return Err(MidiError::new(&msg));
            }
        };

        let time_division = parser_data.get_header().get_time_division();
        let track_count = parser_data.get_track_count();
        let tracks = Vec::with_capacity(track_count);

        let mut ret = Self { tracks, time_division };

        for (i, parser_track) in parser_data.iter_tracks().enumerate() {
            let track = Track::from_track_chunk(parser_track, i);
            ret.tracks.push(track);
        }

        Ok(ret)
    }

    pub fn get_tracks(&self) -> &Vec<Track> {
        &self.tracks
    }

    pub fn get_notes_on_absolute(
        &self,
        track_number: usize,
        channel_number: Option<usize>,
        milliseconds_read: usize
    ) -> MidiResult<HashSet<u8>> {
        let track = match self.tracks.get(track_number) {
            Some(track) => track,
            None => {
                let msg = "Tried to get notes for non-existient track";
                return Err(MidiError::new(msg));
            }
        };
        let tick_position = milliseconds_read * track.ticks_per_second(self.time_division) / 1000;

        track.get_notes_on_absolute(channel_number, tick_position)
    }

    pub fn get_notes_delta(
        &self,
        track_number: usize,
        channel_number: Option<usize>,
        start_time_milliseconds: usize, 
        end_time_milliseconds: usize
    ) -> MidiResult<NoteDelta> {
        let track = match self.tracks.get(track_number) {
            Some(track) => track,
            None => {
                let msg = "Tried to get notes for non-existient track";
                return Err(MidiError::new(msg));
            }
        };

        // x milliseconds * y ticks
        // ------------------------ = number of ticks in x millsconds
        //     1000 milliseconds
        let ticks_per_second = track.ticks_per_second(self.time_division);
        let start_tick = start_time_milliseconds * ticks_per_second / 1000;
        let end_tick = end_time_milliseconds * ticks_per_second / 1000;

        track.get_notes_delta(channel_number, ticks_per_second, start_tick, end_tick)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    track_number: usize,
    track_name: String,
    sequence_number: Option<u16>,
    instrument_name: String,
    tempo: u32,
    channels: Vec<Channel>,
}

impl Track {
    fn from_track_chunk(track_chunk: &parser::TrackChunk, track_number: usize) -> Self{
        let mut channels = Vec::with_capacity(16);
        for i in 0..channels.capacity() {
            channels.push(Channel::new(i as u8));
        }

        let mut ret = Self {
            track_number,
            track_name: String::new(),
            sequence_number: None,
            instrument_name: String::new(),
            tempo: DEFAULT_TEMPO,
            channels
        };

        let mut meta_event_channel_prefix = Option::<usize>::None;
        let mut previous_event_time_offset = 0_usize;
        for event in track_chunk.iter_events() {
            let time_delta = event.get_delta_time();
            let time_offset = previous_event_time_offset + time_delta;
            previous_event_time_offset = time_offset;

            match event.get_event_body() {
                parser::event::EventBody::Channel(parser_channel_event) => {
                    // Put any channel events into their respective channels
                    let channel = parser_channel_event.get_channel() as usize;
                    let channel_event = ChannelEvent::new(time_offset, parser_channel_event);
                    ret.channels[channel].add_event(channel_event);
                },

                parser::event::EventBody::Meta(parser_meta_event) => {
                    use parser::event::meta::MetaEvent;
                    match parser_meta_event {
                        // TODO: is it possible to specifiy these more than once per
                        // track. E.g. change tempo in the middle of a track?
                        MetaEvent::SequenceNumber { number } => {
                            ret.sequence_number = Some(*number);
                        }
                        MetaEvent::SequenceOrTrackName { text } => {
                            ret.track_name = text.to_string();
                        }
                        MetaEvent::InstrumentName { text } => {
                            match meta_event_channel_prefix {
                                Some(channel) => ret.channels[channel].set_instrument_name(text),
                                None => ret.instrument_name = text.to_string()
                            }
                        }
                        MetaEvent::MidiChannelPrefix { channel } => {
                            meta_event_channel_prefix = Some(*channel as usize)
                        }
                        MetaEvent::SetTempo { tempo } => {
                            ret.tempo = *tempo;
                        }
                        _ => {
                            // We only care about the above meta events.
                        }
                    }
                },

                parser::event::EventBody::System(_parser_system_event) => {
                    // We don't care about system events
                }
            }
        }

        ret
    }

    fn get_notes_on_absolute(&self, channel_number: Option<usize>, tick_position: usize) -> MidiResult<HashSet<u8>> {
        if let Some(channel_number) = channel_number {
            let channel = match self.channels.get(channel_number) {
                Some(channel) => channel,
                None => {
                    let msg = "Tried to get notes for non-existent channel";
                    return Err(MidiError::new(msg));
                }
            };

            Ok(channel.get_notes_on_absolute(tick_position))
        }
        else {
            let mut combined = HashSet::new();
            for channel in self.channels.iter() {
                combined = combined.union(&channel.get_notes_on_absolute(tick_position)).cloned().collect();
            }
            Ok(combined)
        }
    }

    fn get_notes_delta(
        &self, channel_number: Option<usize>, ticks_per_second: usize,
        old_tick_position: usize, new_tick_position: usize
    ) -> MidiResult<NoteDelta> {
        if let Some(channel_number) = channel_number {
            let channel = match self.channels.get(channel_number) {
                Some(channel) => channel,
                None => {
                    let msg = "Tried to get notes for non-existent channel";
                    return Err(MidiError::new(msg));
                }
            };

            Ok(channel.get_notes_delta(ticks_per_second, old_tick_position, new_tick_position))
        }
        else {
            let mut channel_deltas = vec![NoteDelta::new(ticks_per_second); 16];
            for (i, channel) in self.channels.iter().enumerate() {
                let channel_delta = channel.get_notes_delta(ticks_per_second, old_tick_position, new_tick_position);
                channel_deltas[i] = channel_delta;
            }

            let merged_iter = channel_deltas.iter().flat_map(|channel_delta| &channel_delta.delta);
            let mut merged_delta: Vec<NoteEvent> = merged_iter.cloned().collect();
            merged_delta.sort_by_cached_key(|k| k.time_offset);
            Ok(NoteDelta { ticks_per_second, delta: merged_delta })
        }
    }

    fn ticks_per_second(&self, time_division: parser::TimeDivision) -> usize {
        match time_division {
            parser::TimeDivision::FramesPerSecond{ frames_per_second, ticks_per_frame } => {
                frames_per_second as usize * ticks_per_frame as usize
            }
            parser::TimeDivision::TicksPerBeat(ticks_per_beat) => {
                let beats_per_minute = (MICROSECONDS_PER_MINUTE / self.tempo) as usize;
                ticks_per_beat as usize * beats_per_minute / 60
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Channel {
    number: u8,
    instrument_name: String,
    note_events: Vec<NoteEvent>,
}

impl Channel {
    fn new(number: u8) -> Self {
        let instrument_name = String::new();
        let note_events = Vec::new();

        Self {
            number,
            instrument_name,
            note_events
        }
    }

    fn set_instrument_name(&mut self, instrument_name: &str) {
        self.instrument_name = instrument_name.to_string();
    }

    fn add_event(&mut self, event: ChannelEvent) {
        match event {
            ChannelEvent::Note(note_event) => self.note_events.push(note_event),
            ChannelEvent::Unused => {
                // not an event we care about
            }
        }
    }

    fn get_notes_on_absolute(&self, tick_position: usize) -> HashSet<u8> {
        let mut notes_on = HashSet::new();
        for event in self.note_events.iter() {
            if event.time_offset > tick_position {
                return notes_on;
            }
            match event.event_type {
                NoteEventType::On => {
                    notes_on.insert(event.note_number);
                }
                NoteEventType::Off => {
                    notes_on.remove(&event.note_number);
                }
            }
        }
        notes_on
    }

    fn get_notes_delta(&self, ticks_per_second: usize, old_position: usize, new_position: usize) -> NoteDelta {
        // Get the index to where we're starting
        debug_assert!(old_position < new_position);

        let start_note_index = self.calculate_next_note_index_from_time_delta(old_position);

        // Gather note events
        let mut note_events = Vec::new();
        for i in start_note_index..self.note_events.len() {
            let note_event = self.note_events[i].clone();
            if note_event.time_offset > new_position {
                break;
            }
            note_events.push(note_event);
        }

        NoteDelta{ ticks_per_second, delta: note_events }
    }

    fn calculate_next_note_index_from_time_delta(&self, time_delta: usize) -> usize {
        let search_result = self.note_events.binary_search_by(|a: &NoteEvent| {
            a.time_offset.cmp(&time_delta)
        });

        match search_result {
            Ok(index) => {
                // found an index that has the exact time delta we want but since this was a binary search
                // there could be other note events with the same time delta that happened before. We need
                // to find the first one
                for i in (0..index).rev() {
                    if self.note_events[i].time_offset < time_delta {
                        return i + 1;
                    }
                }
                0
            }
            Err(index) => {
                // We didn't find an exact mach but thankfully we have the index of where one should be inserted
                // which just so happens to be the index we want
                index
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteDelta {
    ticks_per_second: usize,
    pub delta: Vec<NoteEvent>
}

impl NoteDelta {
    pub fn new(ticks_per_second: usize) -> Self {
        Self { ticks_per_second, delta: Vec::new() }
    }

    pub fn iter(&self) -> std::slice::Iter<NoteEvent> {
        self.delta.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<NoteEvent> {
        self.delta.iter_mut()
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ChannelEvent {
    Unused,
    Note(NoteEvent)
}

impl ChannelEvent {
    fn new(time_offset: usize, parser_event: &parser::event::channel::MidiChannelEvent) -> Self {
        use parser::event::channel::ChannelEventBody;
        let parser_event_body = parser_event.get_inner_event();
        match parser_event_body {
            &ChannelEventBody::NoteOn{ note, velocity } =>
                ChannelEvent::Note(NoteEvent::new(time_offset, NoteEventType::On, note, velocity)),
            &ChannelEventBody::NoteOff{ note, velocity } =>
                ChannelEvent::Note(NoteEvent::new(time_offset, NoteEventType::Off, note, velocity)),
            _ => ChannelEvent::Unused
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteEventType {
    On,
    Off,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteEvent {
    time_offset: usize,
    event_type: NoteEventType,
    note_number: u8,
    velocity: u8
}

impl NoteEvent {
    pub fn new(time_offset: usize, event_type: NoteEventType, note_number: u8, velocity: u8) -> Self {
        Self { time_offset, event_type, note_number, velocity }
    }

    pub fn get_time_offset(&self) -> usize {
        self.time_offset
    }

    pub fn get_event_type(&self) -> NoteEventType {
        self.event_type
    }

    pub fn get_note_number(&self) -> u8 {
        self.note_number
    }

    pub fn get_velocity(&self) -> u8 {
        self.velocity
    }

    pub fn get_time_in_milliseconds(&self, note_delta: &NoteDelta) -> usize {
        tick_position_to_milliseconds(self.time_offset, note_delta.ticks_per_second)
    }
}

pub fn tick_position_to_milliseconds(target_tick_position: usize, ticks_per_second: usize) -> usize {
    target_tick_position * 1000 / ticks_per_second
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_util;

    fn get_test_midi_data() -> MidiResult<MidiData> {
        let path = test_util::get_test_midi_file_path();
        MidiData::from_file(path)
    }

    #[test]
    fn parse_midi_file() {
        let midi_data = match get_test_midi_data() {
            Ok(midi_data) => midi_data,
            Err(err) => panic!("Failed to parse midi: {}", err)
        };

        let tracks = midi_data.get_tracks();
        assert!(tracks.len() == 1);
    }

    #[test]
    fn get_notes_delta_with_channel() {
        let midi_data = match get_test_midi_data() {
            Ok(midi_data) => midi_data,
            Err(err) => panic!("Failed to parse midi: {}", err)
        };

        let delta = match midi_data.get_notes_delta(0, Some(2), 0, 10_000) {
            Ok(delta) => delta,
            Err(err) => {
                panic!("Failed to get note delta: {}", err);
            }
        };
        assert_ne!(delta.delta.len(), 0, "Expected to get notes back");

        let mut notes_on = HashSet::<u8>::new();
        for note_event in delta.delta {
            if note_event.event_type == NoteEventType::On {
                notes_on.insert(note_event.note_number);
            }
            else if note_event.event_type == NoteEventType::Off {
                notes_on.remove(&note_event.note_number);
            }
        }
        assert_eq!(notes_on.len(), 0, "Expected every note that was on to have an off counterpart");
    }

    #[test]
    fn get_notes_on_absolute_with_channel() {
        let midi_data = match get_test_midi_data() {
            Ok(midi_data) => midi_data,
            Err(err) => panic!("Failed to parse midi: {}", err)
        };

        let ticks_per_second = midi_data.get_tracks()[0].ticks_per_second(midi_data.time_division);
        let target_milliseconds = tick_position_to_milliseconds(5100, ticks_per_second);
        
        let notes_on = match midi_data.get_notes_on_absolute(0, Some(2), target_milliseconds) {
            Ok(notes_on) => notes_on,
            Err(err) => {
                panic!("Failed to get notes on: {}", err);
            }
        };

        assert_eq!(notes_on.len(), 1, "Expected there to be one note on");
        assert!(notes_on.contains(&36), "Expected note 36 to be on");
    }
}