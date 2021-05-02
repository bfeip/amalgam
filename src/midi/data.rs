use super::parser;
use super::error::*;

use std::collections::HashSet;

pub struct MidiData<'data> {
    parser_data: parser::MidiData,
    tracks: Vec<Track<'data>>
}

impl<'data> MidiData<'data> {
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> MidiResult<Self> {
        let parser_data = match parser::MidiData::from_file(path) {
            Ok(parser_data) => parser_data,
            Err(err) => {
                let msg = format!("Failed to parse MIDI file: {}", err);
                return Err(MidiError::new(&msg));
            }
        };

        let ret = Self { parser_data, tracks: Vec::new() };

        let track_count = ret.parser_data.get_track_count();
        let mut tracks = Vec::with_capacity(track_count);
        for (i, parser_track) in ret.parser_data.iter_tracks().enumerate() {
            let track = Track::from_track_chunk(parser_track, i);
            tracks.push(track);
        }

        Ok(ret)
    }

    pub fn get_tracks(&self) -> &Vec<Track<'data>> {
        &self.tracks
    }

    pub fn get_notes_on_absolute(
        &self,
        track_number: usize,
        channel_number: usize,
        tick_position: usize
    ) -> MidiResult<HashSet<u8>> {
        let track = match self.tracks.get(track_number) {
            Some(track) => track,
            None => {
                let msg = "Tried to get notes for non-existient track";
                return Err(MidiError::new(msg));
            }
        };

        track.get_notes_on_absolute(channel_number, tick_position)
    }
}

pub struct Track<'data> {
    track_number: usize,
    track_name: &'data str,
    sequence_number: Option<u16>,
    instrument_name: &'data str,
    tempo: u32,
    channels: Vec<Channel<'data>>,
}

impl<'data> Track<'data> {
    fn from_track_chunk(track_chunk: &'data parser::TrackChunk, track_number: usize) -> Self{
        let mut channels = Vec::with_capacity(16);
        for i in 0..channels.len() {
            channels[i] = Channel::new(i as u8);
        }

        let mut ret = Self {
            track_number,
            track_name: "",
            sequence_number: None,
            instrument_name: "",
            tempo: 120,
            channels
        };

        let mut meta_event_channel_prefix = Option::<usize>::None;
        for event in track_chunk.iter_events() {
            let time_delta = event.get_delta_time();
            match event.get_event_body() {
                parser::event::EventBody::Channel(parser_channel_event) => {
                    // Put any channel events into their respective channels
                    let channel = parser_channel_event.get_channel() as usize;
                    let channel_event = ChannelEvent::new(time_delta, parser_channel_event);
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
                            ret.track_name = text;
                        }
                        MetaEvent::InstrumentName { text } => {
                            match meta_event_channel_prefix {
                                Some(channel) => ret.channels[channel].set_instrument_name(text),
                                None => ret.instrument_name = text
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

    fn get_notes_on_absolute(&self, channel_number: usize, tick_position: usize) -> MidiResult<HashSet<u8>> {
        let channel = match self.channels.get(channel_number) {
            Some(channel) => channel,
            None => {
                let msg = "Tried to get notes for non-existent channel";
                return Err(MidiError::new(msg));
            }
        };

        Ok(channel.get_notes_on_absolute(tick_position))
    }

    fn ticks_per_second(&self, owner: &MidiData) -> usize {
        let time_division = owner.parser_data.get_header().get_time_division();
        match time_division {
            parser::TimeDivision::FramesPerSecond{ frames_per_second, ticks_per_frame } => {
                *frames_per_second as usize * *ticks_per_frame as usize
            }
            parser::TimeDivision::TicksPerBeat(ticks_per_beat) => {
                let beats_per_minute = self.tempo as usize;
                *ticks_per_beat as usize * beats_per_minute / 60
            }
        }
    }
}

struct Channel<'data> {
    number: u8,
    instrument_name: &'data str,
    note_events: Vec<NoteEvent>,
}

impl<'data> Channel<'data> {
    fn new(number: u8) -> Self {
        let instrument_name = "";
        let note_events = Vec::new();

        Self {
            number,
            instrument_name,
            note_events,
        }
    }

    fn set_instrument_name(&mut self, instrument_name: &'data str) {
        self.instrument_name = instrument_name;
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
            if event.time_delta > tick_position {
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
}

pub struct NoteDelta {
    pub on: HashSet<u8>,
    pub off: HashSet<u8>
}

enum ChannelEvent {
    Unused,
    Note(NoteEvent)
}

impl ChannelEvent {
    fn new(time_delta: usize, parser_event: &parser::event::channel::MidiChannelEvent) -> Self {
        use parser::event::channel::ChannelEventBody;
        let parser_event_body = parser_event.get_inner_event();
        match parser_event_body {
            &ChannelEventBody::NoteOn{ note, velocity } =>
                ChannelEvent::Note(NoteEvent::new(time_delta, NoteEventType::On, note, velocity)),
            &ChannelEventBody::NoteOff{ note, velocity } =>
                ChannelEvent::Note(NoteEvent::new(time_delta, NoteEventType::Off, note, velocity)),
            _ => ChannelEvent::Unused
        }
    }
}

enum NoteEventType {
    On,
    Off
}

struct NoteEvent {
    time_delta: usize,
    event_type: NoteEventType,
    note_number: u8,
    velocity: u8 
}

impl NoteEvent {
    fn new(time_delta: usize, event_type: NoteEventType, note_number: u8, velocity: u8) -> Self {
        Self { time_delta, event_type, note_number, velocity }
    }
}