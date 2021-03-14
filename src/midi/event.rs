pub mod channel;
pub mod system;
pub mod meta;

use std::io;

use super::error::*;
use super::parse_variable_length;
use self::channel::MidiChannelEvent;
use self::system::MidiSystemEvent;
use self::meta::MidiMetaEvent;

#[derive(Debug, Clone, PartialEq)]
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

    pub fn to_byte(&self) -> u8 {
        match self {
            MidiEventType::NoteOff           => 0x08,
            MidiEventType::NoteOn            => 0x09,
            MidiEventType::NoteAftertouch    => 0x0A,
            MidiEventType::Controller        => 0x0B,
            MidiEventType::ProgramChange     => 0x0C,
            MidiEventType::ChannelAftertouch => 0x0D,
            MidiEventType::PitchBend         => 0x0E,
            MidiEventType::MetaOrSystem      => 0x0F
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MidiEventBody {
    Channel(MidiChannelEvent),
    Meta(MidiMetaEvent),
    System(MidiSystemEvent)
}

#[derive(Debug, Clone, PartialEq)]
pub struct MidiEvent {
    delta_time: usize,
    inner_event: MidiEventBody
}

impl MidiEvent {
    pub fn parse<T: io::Read>(mut midi_stream: T, divided_event_bytes: &mut Vec<u8>) -> MidiResult<Self> {
        let delta_time = match parse_variable_length(&mut midi_stream) {
            Ok(delta_time) => delta_time,
            Err(err) => {
                let msg = format!("Failed to get delta-time for event: {}", err);
                return Err(MidiError::new(&msg));
            }
        };
    
        let mut event_type_and_channel_byte: [u8; 1] = [0; 1];
        read_with_eof_check!(midi_stream, &mut event_type_and_channel_byte); 
        let event_type = match MidiEventType::from_nybble(event_type_and_channel_byte[0] >> 4) {
            Ok(event_type) => event_type,
            Err(err) => {
                let msg = format!("Failed to get event type from nybble: {}", err);
                return Err(MidiError::new(&msg));
            }
        };
        let midi_channel = event_type_and_channel_byte[0] & 0xF;
    
        let inner_event = match event_type {
            MidiEventType::MetaOrSystem => {
                match Self::parse_meta_or_system_event(
                    midi_stream, event_type_and_channel_byte[0], divided_event_bytes
                ) {
                    Ok(event) => event,
                    Err(err) => {
                        let msg = format!("Failed to parse meta or system event: {}", err);
                        return Err(MidiError::new(&msg));
                    }
                }
            },
            _ => {
                let event_result = MidiChannelEvent::parse(midi_stream, event_type, midi_channel);
                match event_result {
                    Ok(event) => MidiEventBody::Channel(event),
                    Err(err) => {
                        let msg = format!("Failed to parse channel event: {}", err);
                        return Err(MidiError::new(&msg));
                    }
                }
            }
        };

        Ok(MidiEvent { delta_time, inner_event })
    }
    
    
    fn parse_meta_or_system_event<T: io::Read>(
        midi_stream: T, event_type_and_channel_byte: u8, divided_event_bytes: &mut Vec<u8>
    ) -> MidiResult<MidiEventBody> {
        match event_type_and_channel_byte {
            0xFF => {
                match MidiMetaEvent::parse(midi_stream) {
                    Ok(meta_event) => return Ok(MidiEventBody::Meta(meta_event)),
                    Err(err) => {
                        let msg = format!("Failed to parse meta event: {}", err);
                        return Err(MidiError::new(&msg));
                    }
                };
            },
            0xF7 | 0xF0 => {
                match MidiSystemEvent::parse(midi_stream, event_type_and_channel_byte, divided_event_bytes) {
                    Ok(system_event) => return Ok(MidiEventBody::System(system_event)),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_note_off() {
        let expected_event = MidiEvent {
            delta_time: 42,
            inner_event: MidiEventBody::Channel(
                MidiChannelEvent::new(8, channel::MidiChannelEventBody::NoteOff {
                    note: 1,
                    velocity: 2
                })
            )
        };

        let bytes: &[u8] = &[42, 0x88, 1, 2];
        let event = match MidiEvent::parse(bytes, &mut Vec::new()) {
            Ok(event) => event,
            Err(err) => {
                panic!("Failed to parse event: {}", err);
            }
        };
        assert_eq!(event, expected_event, "Event does not match expected");
    }

    #[test]
    fn parse_note_on() {
        let expected_event = MidiEvent {
            delta_time: 42,
            inner_event: MidiEventBody::Channel(
                MidiChannelEvent::new(8, channel::MidiChannelEventBody::NoteOn {
                    note: 1,
                    velocity: 2
                })
            )
        };

        let bytes: &[u8] = &[42, 0x98, 1, 2];
        let event = match MidiEvent::parse(bytes, &mut Vec::new()) {
            Ok(event) => event,
            Err(err) => {
                panic!("Failed to parse event: {}", err);
            }
        };
        assert_eq!(event, expected_event, "Event does not match expected");
    }

    #[test]
    fn parse_note_aftertouch() {
        let expected_event = MidiEvent {
            delta_time: 42,
            inner_event: MidiEventBody::Channel(
                MidiChannelEvent::new(8, channel::MidiChannelEventBody::NoteAftertouch {
                    note: 1,
                    amount: 2
                })
            )
        };

        let bytes: &[u8] = &[42, 0xA8, 1, 2];
        let event = match MidiEvent::parse(bytes, &mut Vec::new()) {
            Ok(event) => event,
            Err(err) => {
                panic!("Failed to parse event: {}", err);
            }
        };
        assert_eq!(event, expected_event, "Event does not match expected");
    }

    #[test]
    fn parse_controller() {
        let expected_event = MidiEvent {
            delta_time: 42,
            inner_event: MidiEventBody::Channel(
                MidiChannelEvent::new(8, channel::MidiChannelEventBody::Controller {
                    controller_event: channel::MidiControllerEvent::Modulation,
                    value: 2
                })
            )
        };

        let bytes: &[u8] = &[42, 0xB8, 1, 2];
        let event = match MidiEvent::parse(bytes, &mut Vec::new()) {
            Ok(event) => event,
            Err(err) => {
                panic!("Failed to parse event: {}", err);
            }
        };
        assert_eq!(event, expected_event, "Event does not match expected");
    }

    #[test]
    fn parse_program_change() {
        let expected_event = MidiEvent {
            delta_time: 42,
            inner_event: MidiEventBody::Channel(
                MidiChannelEvent::new(8, channel::MidiChannelEventBody::ProgramChange {
                    program_number: 1
                })
            )
        };

        let bytes: &[u8] = &[42, 0xC8, 1, 2];
        let event = match MidiEvent::parse(bytes, &mut Vec::new()) {
            Ok(event) => event,
            Err(err) => {
                panic!("Failed to parse event: {}", err);
            }
        };
        assert_eq!(event, expected_event, "Event does not match expected");
    }

    #[test]
    fn parse_channel_aftertouch() {
        let expected_event = MidiEvent {
            delta_time: 42,
            inner_event: MidiEventBody::Channel(
                MidiChannelEvent::new(8, channel::MidiChannelEventBody::ChannelAftertouch {
                    amount: 1
                })
            )
        };

        let bytes: &[u8] = &[42, 0xD8, 1, 2];
        let event = match MidiEvent::parse(bytes, &mut Vec::new()) {
            Ok(event) => event,
            Err(err) => {
                panic!("Failed to parse event: {}", err);
            }
        };
        assert_eq!(event, expected_event, "Event does not match expected");
    }

    #[test]
    fn parse_pitch_bend() {
        let expected_event = MidiEvent {
            delta_time: 42,
            inner_event: MidiEventBody::Channel(
                MidiChannelEvent::new(8, channel::MidiChannelEventBody::PitchBend {
                    value: 0x3FFF
                })
            )
        };

        let bytes: &[u8] = &[42, 0xE8, 0x7F, 0x7F];
        let event = match MidiEvent::parse(bytes, &mut Vec::new()) {
            Ok(event) => event,
            Err(err) => {
                panic!("Failed to parse event: {}", err);
            }
        };
        assert_eq!(event, expected_event, "Event does not match expected");
    }

    #[test]
    fn parse_system() {
        let expected_event = MidiEvent {
            delta_time: 0,
            inner_event: MidiEventBody::System(
                MidiSystemEvent::Normal(vec![1, 2])
            )
        };

        let bytes: &[u8] = &[0, 0xF0, 3, 1, 2, 0xF7];
        let event = match MidiEvent::parse(bytes, &mut Vec::new()) {
            Ok(event) => event,
            Err(err) => {
                panic!("Failed to parse event: {}", err);
            }
        };
        assert_eq!(event, expected_event, "Event does not match expected");
    }

    #[test]
    fn parse_meta() {
        let expected_event = MidiEvent {
            delta_time: 0,
            inner_event: MidiEventBody::Meta(MidiMetaEvent::EndOfTrack)
        };

        let bytes: &[u8] = &[0, 0xFF, 0x2F, 0];
        let event = match MidiEvent::parse(bytes, &mut Vec::new()) {
            Ok(event) => event,
            Err(err) => {
                panic!("Failed to parse event: {}", err);
            }
        };
        assert_eq!(event, expected_event, "Event does not match expected");
    }
}