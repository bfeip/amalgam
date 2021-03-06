pub mod channel;
pub mod system;
pub mod meta;

use std::io;

use super::error::*;
use super::{parse_variable_length};
use self::channel::MidiChannelEvent;
use self::system::MidiSystemEvent;
use self::meta::MidiMetaEvent;

pub enum MidiEventType {
    NoteOff           = 0x8,
    NoteOn            = 0x9,
    NoteAftertouch    = 0xA,
    Controller        = 0xB,
    ProgramChange     = 0xC,
    ChannelAftertouch = 0xD,
    PitchBend         = 0xE,
    MetaOrSystem      = 0xF
}

impl MidiEventType {
    pub fn from_nybble(nybble: u8) -> MidiResult<Self> {
        let event_type = match nybble & 0xF {
            0x8 => MidiEventType::NoteOff,
            0x9 => MidiEventType::NoteOn,
            0xA => MidiEventType::NoteAftertouch,
            0xB => MidiEventType::Controller,
            0xC => MidiEventType::ProgramChange,
            0xD => MidiEventType::ChannelAftertouch,
            0xE => MidiEventType::PitchBend,
            0xF => MidiEventType::MetaOrSystem,
            _   => {
                let msg = format!("Unknown MIDI event type {:#03x}", nybble);
                return Err(MidiError::new(&msg))
            }
        };
        Ok(event_type)
    }
}

pub enum MidiEvent {
    Channel(MidiChannelEvent),
    Meta(MidiMetaEvent),
    System(MidiSystemEvent)
}

impl MidiEvent {
    pub fn parse<T: io::Read>(midi_file_stream: &mut T, divided_event_bytes: &mut Vec<u8>) -> MidiResult<MidiEvent> {
        let delta_time = match parse_variable_length(midi_file_stream) {
            Ok(delta_time) => delta_time,
            Err(err) => {
                let msg = format!("Failed to get delta-time for event: {}", err);
                return Err(MidiError::new(&msg));
            }
        };
    
        let mut event_type_and_channel_byte: [u8; 1] = [0; 1];
        read_with_eof_check!(midi_file_stream, &mut event_type_and_channel_byte); 
        let event_type = match MidiEventType::from_nybble(event_type_and_channel_byte[0] >> 4) {
            Ok(event_type) => event_type,
            Err(err) => {
                let msg = format!("Failed to get event type from nybble: {}", err);
                return Err(MidiError::new(&msg));
            }
        };
        let midi_channel = event_type_and_channel_byte[0] & 0xF;
    
        match event_type {
            MidiEventType::MetaOrSystem => {
                Self::parse_meta_or_system_event(
                    midi_file_stream, event_type_and_channel_byte[0], divided_event_bytes
                )
            },
            _ => {
                let mut param1: [u8; 1] = [0; 1];
                let mut param2: [u8; 1] = [0; 1];
                read_with_eof_check!(midi_file_stream, &mut param1);
                read_with_eof_check!(midi_file_stream, &mut param2);
                let channel_event = MidiChannelEvent::new(
                    delta_time, event_type, midi_channel, param1[0], param2[0]
                );
                Ok(MidiEvent::Channel(channel_event))
            }
        }
    }
    
    
    fn parse_meta_or_system_event<T: io::Read>(
        midi_file_stream: &mut T, event_type_and_channel_byte: u8, divided_event_bytes: &mut Vec<u8>
    ) -> MidiResult<MidiEvent> {
        match event_type_and_channel_byte {
            0xFF => {
                match MidiMetaEvent::parse(midi_file_stream) {
                    Ok(meta_event) => return Ok(MidiEvent::Meta(meta_event)),
                    Err(err) => {
                        let msg = format!("Failed to parse meta event: {}", err);
                        return Err(MidiError::new(&msg));
                    }
                };
            },
            0xF7 | 0xF0 => {
                match MidiSystemEvent::parse(midi_file_stream, event_type_and_channel_byte, divided_event_bytes) {
                    Ok(system_event) => return Ok(MidiEvent::System(system_event)),
                    Err(err) => {
                        let msg = format!("Failed to parse system event: {}", err);
                        return Err(MidiError::new(&msg));
                    }
                };
            }
            _ => {
                let msg = format!(
                    "Tried to parse meta or system event but got unknown type byte {:#04x}",
                    event_type_and_channel_byte
                );
                return Err(MidiError::new(&msg));
            }
        }
    }
}