use std::io;
use super::super::{parse_variable_length, parse_string};
use crate::midi::error::*;

pub enum MetaEventType {
    SequenceNumber,
    TextEvent,
    CopyrightNotice,
    SequenceOrTrackName,
    InstrumentName,
    Lyrics,
    Marker,
    CuePoint,
    MidiChannelPrefix,
    EndOfTrack,
    SetTempo,
    SmpteOffset,
    TimeSignature,
    KeySignature,
    SequencerSpecific
}

impl MetaEventType {
    pub fn from_byte(byte: u8) -> MidiResult<Self> {
        let event_type = match byte {
            0x00 => MetaEventType::SequenceNumber,
            0x01 => MetaEventType::TextEvent,
            0x02 => MetaEventType::CopyrightNotice,
            0x03 => MetaEventType::SequenceOrTrackName,
            0x04 => MetaEventType::InstrumentName,
            0x05 => MetaEventType::Lyrics,
            0x06 => MetaEventType::Marker,
            0x07 => MetaEventType::CuePoint,
            0x20 => MetaEventType::MidiChannelPrefix,
            0x2F => MetaEventType::EndOfTrack,
            0x51 => MetaEventType::SetTempo,
            0x54 => MetaEventType::SmpteOffset,
            0x58 => MetaEventType::TimeSignature,
            0x59 => MetaEventType::KeySignature,
            0x7F => MetaEventType::SequencerSpecific,
            _ => {
                let msg = format!("Unknown MIDI event type {:#04x}", byte);
                return Err(MidiError::new(&msg));
            }
        };
        Ok(event_type)
    }

    pub fn to_byte(&self) -> u8 {
        match self {
            MetaEventType::SequenceNumber      => 0x00,
            MetaEventType::TextEvent           => 0x01,
            MetaEventType::CopyrightNotice     => 0x02,
            MetaEventType::SequenceOrTrackName => 0x03,
            MetaEventType::InstrumentName      => 0x04,
            MetaEventType::Lyrics              => 0x05,
            MetaEventType::Marker              => 0x06,
            MetaEventType::CuePoint            => 0x07,
            MetaEventType::MidiChannelPrefix   => 0x20,
            MetaEventType::EndOfTrack          => 0x2F,
            MetaEventType::SetTempo            => 0x51,
            MetaEventType::SmpteOffset         => 0x54,
            MetaEventType::TimeSignature       => 0x58,
            MetaEventType::KeySignature        => 0x59,
            MetaEventType::SequencerSpecific   => 0x7F
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum MetaEvent {
    SequenceNumber { number: u16 },
    TextEvent { text: String },
    CopyrightNotice { text: String },
    SequenceOrTrackName { text: String },
    InstrumentName { text: String },
    Lyrics { text: String },
    Marker { text: String },
    CuePoint { text: String },
    MidiChannelPrefix { channel: u8 },
    EndOfTrack,
    SetTempo { tempo: u32 },
    SmpteOffset { hour: u8, min: u8, sec: u8, frame: u8, sub_frame: u8 },
    TimeSignature { numerator: u8, denominator: u8, metro: u8, thirty_seconds: u8},
    KeySignature { key: u8, scale: u8 },
    SequencerSpecific { data: usize }
}

impl MetaEvent {
    pub fn parse<T: io::Read>(mut midi_stream: T) -> MidiResult<MetaEvent> {
        let mut meta_event_type_byte: [u8; 1] = [0; 1];
        read_with_eof_check!(midi_stream, &mut meta_event_type_byte);
        
        let size = match parse_variable_length(&mut midi_stream) {
            Ok(size) => size,
            Err(err) => {
                let msg = format!("Failed to parse MIDI meta event because we couldn't parse size: {}", err);
                return Err(MidiError::new(&msg));
            }
        };
    
        let meta_event_type = match MetaEventType::from_byte(meta_event_type_byte[0]) {
            Ok(meta_event_type) => meta_event_type,
            Err(err) => {
                let msg = format!("Failed to get meta event type: {}", err);
                return Err(MidiError::new(&msg));
            }
        };
        match meta_event_type {
            MetaEventType::SequenceNumber => {
                let mut bytes: [u8; 2] = [0; 2];
                read_with_eof_check!(midi_stream, &mut bytes);
                let number = u16::from_be_bytes(bytes);
                Ok(MetaEvent::SequenceNumber{ number })
            },
    
            MetaEventType::TextEvent => {
                let text = match parse_string(midi_stream, size) {
                    Ok(text) => text,
                    Err(err) => {
                        let msg = format!(
                            "Failed to parse MIDI meta event because we couldn't parse text event: {}", err
                        );
                        return Err(MidiError::new(&msg))
                    }
                };
                Ok(MetaEvent::TextEvent{ text })
            },
    
            MetaEventType::CopyrightNotice => {
                let text = match parse_string(midi_stream, size) {
                    Ok(text) => text,
                    Err(err) => {
                        let msg = format!(
                            "Failed to parse MIDI meta event because we couldn't parse copyright notice: {}", err
                        );
                        return Err(MidiError::new(&msg))
                    }
                };
                Ok(MetaEvent::CopyrightNotice{ text })
            },
    
            MetaEventType::SequenceOrTrackName => {
                let text = match parse_string(midi_stream, size) {
                    Ok(text) => text,
                    Err(err) => {
                        let msg = format!(
                            "Failed to parse MIDI meta event because we couldn't parse sequence or track name: {}", err
                        );
                        return Err(MidiError::new(&msg))
                    }
                };
                Ok(MetaEvent::SequenceOrTrackName{ text })
            },
    
            MetaEventType::InstrumentName => {
                let text = match parse_string(midi_stream, size) {
                    Ok(text) => text,
                    Err(err) => {
                        let msg = format!(
                            "Failed to parse MIDI meta event because we couldn't parse instrument name: {}", err
                        );
                        return Err(MidiError::new(&msg))
                    }
                };
                Ok(MetaEvent::InstrumentName{ text })
            },
    
            MetaEventType::Lyrics => {
                let text = match parse_string(midi_stream, size) {
                    Ok(text) => text,
                    Err(err) => {
                        let msg = format!(
                            "Failed to parse MIDI meta event because we couldn't parse lyrics: {}", err
                        );
                        return Err(MidiError::new(&msg))
                    }
                };
                Ok(MetaEvent::Lyrics{ text })
            },
    
            MetaEventType::Marker => {
                let text = match parse_string(midi_stream, size) {
                    Ok(text) => text,
                    Err(err) => {
                        let msg = format!(
                            "Failed to parse MIDI meta event because we couldn't parse marker: {}", err
                        );
                        return Err(MidiError::new(&msg))
                    }
                };
                Ok(MetaEvent::Marker{ text })
            },
    
            MetaEventType::CuePoint => {
                let text = match parse_string(midi_stream, size) {
                    Ok(text) => text,
                    Err(err) => {
                        let msg = format!(
                            "Failed to parse MIDI meta event because we couldn't parse cue point: {}", err
                        );
                        return Err(MidiError::new(&msg))
                    }
                };
                Ok(MetaEvent::CuePoint{ text })
            },
    
            MetaEventType::MidiChannelPrefix => {
                let mut channel_byte: [u8; 1] = [0; 1];
                read_with_eof_check!(midi_stream, &mut channel_byte);
                Ok(MetaEvent::MidiChannelPrefix{ channel: channel_byte[0] })
            },
    
            MetaEventType::EndOfTrack => Ok(MetaEvent::EndOfTrack),
    
            MetaEventType::SetTempo => {
                // Tempo is represented by 3 bytes in midi but we need 4 bytes to make a u32 so we just split off the
                // first byte that'll always be zero
                let mut tempo_u32_bytes: [u8; 4] = [0; 4];
                let (_zero_byte, mut tempo_bytes) = tempo_u32_bytes.split_first_mut().unwrap();
                read_with_eof_check!(midi_stream, &mut tempo_bytes);
                let tempo = u32::from_be_bytes(tempo_u32_bytes);
                Ok(MetaEvent::SetTempo{ tempo })
            },
    
            MetaEventType::SmpteOffset => {
                let mut bytes: [u8; 5] = [0; 5];
                read_with_eof_check!(midi_stream, &mut bytes);
                Ok(MetaEvent::SmpteOffset {
                    hour: bytes[0],
                    min: bytes[1],
                    sec: bytes[2],
                    frame: bytes[3],
                    sub_frame: bytes[4]
                })
            },
    
            MetaEventType::TimeSignature => {
                let mut bytes: [u8; 4] = [0; 4];
                read_with_eof_check!(midi_stream, &mut bytes);
                Ok(MetaEvent::TimeSignature {
                    numerator: bytes[0],
                    denominator: bytes[1],
                    metro: bytes[2],
                    thirty_seconds: bytes[3]
                })
            },
    
            MetaEventType::KeySignature => {
                let mut bytes: [u8; 2] = [0; 2];
                read_with_eof_check!(midi_stream, &mut bytes);
                Ok(MetaEvent::KeySignature {
                    key: bytes[0],
                    scale: bytes[1]
                })
            }
    
            MetaEventType::SequencerSpecific => {
                // WRONG. This is not variable length. it's just a bunch of bytes until size is fulfilled
                let data = match parse_variable_length(midi_stream) {
                    Ok(data) => data,
                    Err(err) => {
                        let msg = format!("Failed to sequencer specific meta event: {}", err);
                        return Err(MidiError::new(&msg));
                    }
                };
                Ok(MetaEvent::SequencerSpecific{ data })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sequence_number() {
        let expected_event = MetaEvent::SequenceNumber{ number: 8256 };
        let type_byte = MetaEventType::SequenceNumber.to_byte();
        let mut bytes: &[u8] = &[type_byte, 2, 32, 64];
        let event = MetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_text_event() {
        let expected_event = MetaEvent::TextEvent{ text: "text".to_string() };
        let type_byte = MetaEventType::TextEvent.to_byte();
        let mut bytes: &[u8] = &[type_byte, 4, 't' as u8, 'e' as u8, 'x' as u8, 't' as u8];
        let event = MetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_copyright_notice() {
        let expected_event = MetaEvent::CopyrightNotice{ text: "text".to_string() };
        let type_byte = MetaEventType::CopyrightNotice.to_byte();
        let mut bytes: &[u8] = &[type_byte, 4, 't' as u8, 'e' as u8, 'x' as u8, 't' as u8];
        let event = MetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_sequence_or_track_name() {
        let expected_event = MetaEvent::SequenceOrTrackName{ text: "text".to_string() };
        let type_byte = MetaEventType::SequenceOrTrackName.to_byte();
        let mut bytes: &[u8] = &[type_byte, 4, 't' as u8, 'e' as u8, 'x' as u8, 't' as u8];
        let event = MetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_instrument_name() {
        let expected_event = MetaEvent::InstrumentName{ text: "text".to_string() };
        let type_byte = MetaEventType::InstrumentName.to_byte();
        let mut bytes: &[u8] = &[type_byte, 4, 't' as u8, 'e' as u8, 'x' as u8, 't' as u8];
        let event = MetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_lyrics() {
        let expected_event = MetaEvent::Lyrics{ text: "text".to_string() };
        let type_byte = MetaEventType::Lyrics.to_byte();
        let mut bytes: &[u8] = &[type_byte, 4, 't' as u8, 'e' as u8, 'x' as u8, 't' as u8];
        let event = MetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_marker() {
        let expected_event = MetaEvent::Marker{ text: "text".to_string() };
        let type_byte = MetaEventType::Marker.to_byte();
        let mut bytes: &[u8] = &[type_byte, 4, 't' as u8, 'e' as u8, 'x' as u8, 't' as u8];
        let event = MetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_cue_point() {
        let expected_event = MetaEvent::CuePoint{ text: "text".to_string() };
        let type_byte = MetaEventType::CuePoint.to_byte();
        let mut bytes: &[u8] = &[type_byte, 4, 't' as u8, 'e' as u8, 'x' as u8, 't' as u8];
        let event = MetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_midi_channel_prefix() {
        let expected_event = MetaEvent::MidiChannelPrefix{ channel: 42 };
        let type_byte = MetaEventType::MidiChannelPrefix.to_byte();
        let mut bytes: &[u8] = &[type_byte, 1, 42];
        let event = MetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_end_of_track() {
        let expected_event = MetaEvent::EndOfTrack;
        let type_byte = MetaEventType::EndOfTrack.to_byte();
        let mut bytes: &[u8] = &[type_byte, 0];
        let event = MetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_set_tempo() {
        let expected_event = MetaEvent::SetTempo{ tempo: 42 };
        let type_byte = MetaEventType::SetTempo.to_byte();
        let mut bytes: &[u8] = &[type_byte, 3, 0, 0, 42];
        let event = MetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_smpte_offset() {
        let expected_event = MetaEvent::SmpteOffset{ 
            hour: 1,
            min: 2,
            sec: 3,
            frame: 4,
            sub_frame: 5
        };
        let type_byte = MetaEventType::SmpteOffset.to_byte();
        let mut bytes: &[u8] = &[type_byte, 5, 1, 2, 3, 4, 5];
        let event = MetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_time_signature() {
        let expected_event = MetaEvent::TimeSignature{ 
            numerator: 1,
            denominator: 2,
            metro: 3,
            thirty_seconds: 4,
        };
        let type_byte = MetaEventType::TimeSignature.to_byte();
        let mut bytes: &[u8] = &[type_byte, 4, 1, 2, 3, 4];
        let event = MetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_key_signature() {
        let expected_event = MetaEvent::KeySignature{ key: 1, scale: 2 };
        let type_byte = MetaEventType::KeySignature.to_byte();
        let mut bytes: &[u8] = &[type_byte, 2, 1, 2];
        let event = MetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_sequencer_specific() {
        let expected_event = MetaEvent::SequencerSpecific{ data: 42 };
        let type_byte = MetaEventType::SequencerSpecific.to_byte();
        let mut bytes: &[u8] = &[type_byte, 1, 42];
        let event = MetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }
}