pub mod channel;
pub mod system;
pub mod meta;

use std::io;

use crate::{SynthError, SynthResult};
use super::parse_variable_length;
use self::channel::MidiChannelEvent;
use self::system::SystemEvent;
use self::meta::MetaEvent;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventType {
    NoteOff           = 0x8,
    NoteOn            = 0x9,
    NoteAftertouch    = 0xA,
    Controller        = 0xB,
    ProgramChange     = 0xC,
    ChannelAftertouch = 0xD,
    PitchBend         = 0xE,
    MetaOrSystem      = 0xF
}

impl EventType {
    pub fn from_nybble(nybble: u8) -> SynthResult<Self> {
        let event_type = match nybble & 0xF {
            0x8 => EventType::NoteOff,
            0x9 => EventType::NoteOn,
            0xA => EventType::NoteAftertouch,
            0xB => EventType::Controller,
            0xC => EventType::ProgramChange,
            0xD => EventType::ChannelAftertouch,
            0xE => EventType::PitchBend,
            0xF => EventType::MetaOrSystem,
            _   => {
                let msg = format!("Unknown MIDI event type {:#03x}", nybble);
                return Err(SynthError::new(&msg))
            }
        };
        Ok(event_type)
    }

    pub fn to_byte(self) -> u8 {
        match self {
            EventType::NoteOff           => 0x08,
            EventType::NoteOn            => 0x09,
            EventType::NoteAftertouch    => 0x0A,
            EventType::Controller        => 0x0B,
            EventType::ProgramChange     => 0x0C,
            EventType::ChannelAftertouch => 0x0D,
            EventType::PitchBend         => 0x0E,
            EventType::MetaOrSystem      => 0x0F
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventBody {
    Channel(MidiChannelEvent),
    Meta(MetaEvent),
    System(SystemEvent)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    delta_time: usize,
    event_body: EventBody
}

impl Event {
    pub fn parse<T: io::Read>(mut midi_stream: T, divided_event_bytes: &mut Vec<u8>) -> SynthResult<Self> {
        let delta_time = match parse_variable_length(&mut midi_stream) {
            Ok(delta_time) => delta_time,
            Err(err) => {
                let msg = format!("Failed to get delta-time for event: {}", err);
                return Err(SynthError::new(&msg));
            }
        };
    
        let mut event_type_and_channel_byte: [u8; 1] = [0; 1];
        read_with_eof_check!(midi_stream, &mut event_type_and_channel_byte); 
        let event_type = match EventType::from_nybble(event_type_and_channel_byte[0] >> 4) {
            Ok(event_type) => event_type,
            Err(err) => {
                let msg = format!("Failed to get event type from nybble: {}", err);
                return Err(SynthError::new(&msg));
            }
        };
        let midi_channel = event_type_and_channel_byte[0] & 0xF;
    
        let inner_event = match event_type {
            EventType::MetaOrSystem => {
                match Self::parse_meta_or_system_event(
                    midi_stream, event_type_and_channel_byte[0], divided_event_bytes
                ) {
                    Ok(event) => event,
                    Err(err) => {
                        let msg = format!("Failed to parse meta or system event: {}", err);
                        return Err(SynthError::new(&msg));
                    }
                }
            },
            _ => {
                let event_result = MidiChannelEvent::parse(midi_stream, event_type, midi_channel);
                match event_result {
                    Ok(event) => EventBody::Channel(event),
                    Err(err) => {
                        let msg = format!("Failed to parse channel event: {}", err);
                        return Err(SynthError::new(&msg));
                    }
                }
            }
        };

        Ok(Event { delta_time, event_body: inner_event })
    }
    
    
    fn parse_meta_or_system_event<T: io::Read>(
        midi_stream: T, event_type_and_channel_byte: u8, divided_event_bytes: &mut Vec<u8>
    ) -> SynthResult<EventBody> {
        match event_type_and_channel_byte {
            0xFF => {
                match MetaEvent::parse(midi_stream) {
                    Ok(meta_event) => Ok(EventBody::Meta(meta_event)),
                    Err(err) => {
                        let msg = format!("Failed to parse meta event: {}", err);
                        Err(SynthError::new(&msg))
                    }
                }
            },
            0xF7 | 0xF0 => {
                match SystemEvent::parse(midi_stream, event_type_and_channel_byte, divided_event_bytes) {
                    Ok(system_event) => Ok(EventBody::System(system_event)),
                    Err(err) => {
                        let msg = format!("Failed to parse system event: {}", err);
                        Err(SynthError::new(&msg))
                    }
                }
            }
            _ => {
                let msg = format!(
                    "Tried to parse meta or system event but got unknown type byte {:#04x}",
                    event_type_and_channel_byte
                );
                Err(SynthError::new(&msg))
            }
        }
    }

    pub fn get_delta_time(&self) -> usize {
        self.delta_time
    }

    pub fn get_event_body(&self) -> &EventBody {
        &self.event_body
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_note_off() {
        let expected_event = Event {
            delta_time: 42,
            event_body: EventBody::Channel(
                MidiChannelEvent::new(8, channel::ChannelEventBody::NoteOff {
                    note: 1,
                    velocity: 2
                })
            )
        };

        let bytes: &[u8] = &[42, 0x88, 1, 2];
        let event = match Event::parse(bytes, &mut Vec::new()) {
            Ok(event) => event,
            Err(err) => {
                panic!("Failed to parse event: {}", err);
            }
        };
        assert_eq!(event, expected_event, "Event does not match expected");
    }

    #[test]
    fn parse_note_on() {
        let expected_event = Event {
            delta_time: 42,
            event_body: EventBody::Channel(
                MidiChannelEvent::new(8, channel::ChannelEventBody::NoteOn {
                    note: 1,
                    velocity: 2
                })
            )
        };

        let bytes: &[u8] = &[42, 0x98, 1, 2];
        let event = match Event::parse(bytes, &mut Vec::new()) {
            Ok(event) => event,
            Err(err) => {
                panic!("Failed to parse event: {}", err);
            }
        };
        assert_eq!(event, expected_event, "Event does not match expected");
    }

    #[test]
    fn parse_note_aftertouch() {
        let expected_event = Event {
            delta_time: 42,
            event_body: EventBody::Channel(
                MidiChannelEvent::new(8, channel::ChannelEventBody::NoteAftertouch {
                    note: 1,
                    amount: 2
                })
            )
        };

        let bytes: &[u8] = &[42, 0xA8, 1, 2];
        let event = match Event::parse(bytes, &mut Vec::new()) {
            Ok(event) => event,
            Err(err) => {
                panic!("Failed to parse event: {}", err);
            }
        };
        assert_eq!(event, expected_event, "Event does not match expected");
    }

    #[test]
    fn parse_controller() {
        let expected_event = Event {
            delta_time: 42,
            event_body: EventBody::Channel(
                MidiChannelEvent::new(8, channel::ChannelEventBody::Controller {
                    controller_event: channel::ControllerEvent::Modulation,
                    value: 2
                })
            )
        };

        let bytes: &[u8] = &[42, 0xB8, 1, 2];
        let event = match Event::parse(bytes, &mut Vec::new()) {
            Ok(event) => event,
            Err(err) => {
                panic!("Failed to parse event: {}", err);
            }
        };
        assert_eq!(event, expected_event, "Event does not match expected");
    }

    #[test]
    fn parse_program_change() {
        let expected_event = Event {
            delta_time: 42,
            event_body: EventBody::Channel(
                MidiChannelEvent::new(8, channel::ChannelEventBody::ProgramChange {
                    program_number: 1
                })
            )
        };

        let bytes: &[u8] = &[42, 0xC8, 1, 2];
        let event = match Event::parse(bytes, &mut Vec::new()) {
            Ok(event) => event,
            Err(err) => {
                panic!("Failed to parse event: {}", err);
            }
        };
        assert_eq!(event, expected_event, "Event does not match expected");
    }

    #[test]
    fn parse_channel_aftertouch() {
        let expected_event = Event {
            delta_time: 42,
            event_body: EventBody::Channel(
                MidiChannelEvent::new(8, channel::ChannelEventBody::ChannelAftertouch {
                    amount: 1
                })
            )
        };

        let bytes: &[u8] = &[42, 0xD8, 1, 2];
        let event = match Event::parse(bytes, &mut Vec::new()) {
            Ok(event) => event,
            Err(err) => {
                panic!("Failed to parse event: {}", err);
            }
        };
        assert_eq!(event, expected_event, "Event does not match expected");
    }

    #[test]
    fn parse_pitch_bend() {
        let expected_event = Event {
            delta_time: 42,
            event_body: EventBody::Channel(
                MidiChannelEvent::new(8, channel::ChannelEventBody::PitchBend {
                    value: 0x3FFF
                })
            )
        };

        let bytes: &[u8] = &[42, 0xE8, 0x7F, 0x7F];
        let event = match Event::parse(bytes, &mut Vec::new()) {
            Ok(event) => event,
            Err(err) => {
                panic!("Failed to parse event: {}", err);
            }
        };
        assert_eq!(event, expected_event, "Event does not match expected");
    }

    #[test]
    fn parse_system() {
        let expected_event = Event {
            delta_time: 0,
            event_body: EventBody::System(
                SystemEvent::Normal(vec![1, 2])
            )
        };

        let bytes: &[u8] = &[0, 0xF0, 3, 1, 2, 0xF7];
        let event = match Event::parse(bytes, &mut Vec::new()) {
            Ok(event) => event,
            Err(err) => {
                panic!("Failed to parse event: {}", err);
            }
        };
        assert_eq!(event, expected_event, "Event does not match expected");
    }

    #[test]
    fn parse_meta() {
        let expected_event = Event {
            delta_time: 0,
            event_body: EventBody::Meta(MetaEvent::EndOfTrack)
        };

        let bytes: &[u8] = &[0, 0xFF, 0x2F, 0];
        let event = match Event::parse(bytes, &mut Vec::new()) {
            Ok(event) => event,
            Err(err) => {
                panic!("Failed to parse event: {}", err);
            }
        };
        assert_eq!(event, expected_event, "Event does not match expected");
    }
}