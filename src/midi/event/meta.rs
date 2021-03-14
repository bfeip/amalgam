use std::io;
use super::super::{error::*, parse_variable_length, parse_string};

pub enum MidiMetaEventType {
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

impl MidiMetaEventType {
    pub fn from_byte(byte: u8) -> MidiResult<Self> {
        let event_type = match byte {
            0x00 => MidiMetaEventType::SequenceNumber,
            0x01 => MidiMetaEventType::TextEvent,
            0x02 => MidiMetaEventType::CopyrightNotice,
            0x03 => MidiMetaEventType::SequenceOrTrackName,
            0x04 => MidiMetaEventType::InstrumentName,
            0x05 => MidiMetaEventType::Lyrics,
            0x06 => MidiMetaEventType::Marker,
            0x07 => MidiMetaEventType::CuePoint,
            0x20 => MidiMetaEventType::MidiChannelPrefix,
            0x2F => MidiMetaEventType::EndOfTrack,
            0x51 => MidiMetaEventType::SetTempo,
            0x54 => MidiMetaEventType::SmpteOffset,
            0x58 => MidiMetaEventType::TimeSignature,
            0x59 => MidiMetaEventType::KeySignature,
            0x7F => MidiMetaEventType::SequencerSpecific,
            _ => {
                let msg = format!("Unknown MIDI event type {:#04x}", byte);
                return Err(MidiError::new(&msg));
            }
        };
        Ok(event_type)
    }

    pub fn to_byte(&self) -> u8 {
        match self {
            MidiMetaEventType::SequenceNumber      => 0x00,
            MidiMetaEventType::TextEvent           => 0x01,
            MidiMetaEventType::CopyrightNotice     => 0x02,
            MidiMetaEventType::SequenceOrTrackName => 0x03,
            MidiMetaEventType::InstrumentName      => 0x04,
            MidiMetaEventType::Lyrics              => 0x05,
            MidiMetaEventType::Marker              => 0x06,
            MidiMetaEventType::CuePoint            => 0x07,
            MidiMetaEventType::MidiChannelPrefix   => 0x20,
            MidiMetaEventType::EndOfTrack          => 0x2F,
            MidiMetaEventType::SetTempo            => 0x51,
            MidiMetaEventType::SmpteOffset         => 0x54,
            MidiMetaEventType::TimeSignature       => 0x58,
            MidiMetaEventType::KeySignature        => 0x59,
            MidiMetaEventType::SequencerSpecific   => 0x7F
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum MidiMetaEvent {
    SequenceNumber { msb: u8, lsb: u8 },
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

impl MidiMetaEvent {
    pub fn parse<T: io::Read>(mut midi_stream: T) -> MidiResult<MidiMetaEvent> {
        let mut meta_event_type_byte: [u8; 1] = [0; 1];
        read_with_eof_check!(midi_stream, &mut meta_event_type_byte);
        
        let size = match parse_variable_length(&mut midi_stream) {
            Ok(size) => size,
            Err(err) => {
                let msg = format!("Failed to parse MIDI meta event because we couldn't parse size: {}", err);
                return Err(MidiError::new(&msg));
            }
        };
    
        let meta_event_type = match MidiMetaEventType::from_byte(meta_event_type_byte[0]) {
            Ok(meta_event_type) => meta_event_type,
            Err(err) => {
                let msg = format!("Failed to get meta event type: {}", err);
                return Err(MidiError::new(&msg));
            }
        };
        match meta_event_type {
            MidiMetaEventType::SequenceNumber => {
                let mut bytes: [u8; 2] = [0; 2];
                read_with_eof_check!(midi_stream, &mut bytes);
                Ok(MidiMetaEvent::SequenceNumber{ msb: bytes[0], lsb: bytes[1] })
            },
    
            MidiMetaEventType::TextEvent => {
                let text = match parse_string(midi_stream, size) {
                    Ok(text) => text,
                    Err(err) => {
                        let msg = format!(
                            "Failed to parse MIDI meta event because we couldn't parse text event: {}", err
                        );
                        return Err(MidiError::new(&msg))
                    }
                };
                Ok(MidiMetaEvent::TextEvent{ text })
            },
    
            MidiMetaEventType::CopyrightNotice => {
                let text = match parse_string(midi_stream, size) {
                    Ok(text) => text,
                    Err(err) => {
                        let msg = format!(
                            "Failed to parse MIDI meta event because we couldn't parse copyright notice: {}", err
                        );
                        return Err(MidiError::new(&msg))
                    }
                };
                Ok(MidiMetaEvent::CopyrightNotice{ text })
            },
    
            MidiMetaEventType::SequenceOrTrackName => {
                let text = match parse_string(midi_stream, size) {
                    Ok(text) => text,
                    Err(err) => {
                        let msg = format!(
                            "Failed to parse MIDI meta event because we couldn't parse sequence or track name: {}", err
                        );
                        return Err(MidiError::new(&msg))
                    }
                };
                Ok(MidiMetaEvent::SequenceOrTrackName{ text })
            },
    
            MidiMetaEventType::InstrumentName => {
                let text = match parse_string(midi_stream, size) {
                    Ok(text) => text,
                    Err(err) => {
                        let msg = format!(
                            "Failed to parse MIDI meta event because we couldn't parse instrument name: {}", err
                        );
                        return Err(MidiError::new(&msg))
                    }
                };
                Ok(MidiMetaEvent::InstrumentName{ text })
            },
    
            MidiMetaEventType::Lyrics => {
                let text = match parse_string(midi_stream, size) {
                    Ok(text) => text,
                    Err(err) => {
                        let msg = format!(
                            "Failed to parse MIDI meta event because we couldn't parse lyrics: {}", err
                        );
                        return Err(MidiError::new(&msg))
                    }
                };
                Ok(MidiMetaEvent::Lyrics{ text })
            },
    
            MidiMetaEventType::Marker => {
                let text = match parse_string(midi_stream, size) {
                    Ok(text) => text,
                    Err(err) => {
                        let msg = format!(
                            "Failed to parse MIDI meta event because we couldn't parse marker: {}", err
                        );
                        return Err(MidiError::new(&msg))
                    }
                };
                Ok(MidiMetaEvent::Marker{ text })
            },
    
            MidiMetaEventType::CuePoint => {
                let text = match parse_string(midi_stream, size) {
                    Ok(text) => text,
                    Err(err) => {
                        let msg = format!(
                            "Failed to parse MIDI meta event because we couldn't parse cue point: {}", err
                        );
                        return Err(MidiError::new(&msg))
                    }
                };
                Ok(MidiMetaEvent::CuePoint{ text })
            },
    
            MidiMetaEventType::MidiChannelPrefix => {
                let mut channel_byte: [u8; 1] = [0; 1];
                read_with_eof_check!(midi_stream, &mut channel_byte);
                Ok(MidiMetaEvent::MidiChannelPrefix{ channel: channel_byte[0] })
            },
    
            MidiMetaEventType::EndOfTrack => Ok(MidiMetaEvent::EndOfTrack),
    
            MidiMetaEventType::SetTempo => {
                let mut tempo_bytes: [u8; 4] = [0; 4];
                read_with_eof_check!(midi_stream, &mut tempo_bytes);
                let tempo = u32::from_be_bytes(tempo_bytes);
                Ok(MidiMetaEvent::SetTempo{ tempo })
            },
    
            MidiMetaEventType::SmpteOffset => {
                let mut bytes: [u8; 5] = [0; 5];
                read_with_eof_check!(midi_stream, &mut bytes);
                Ok(MidiMetaEvent::SmpteOffset {
                    hour: bytes[0],
                    min: bytes[1],
                    sec: bytes[2],
                    frame: bytes[3],
                    sub_frame: bytes[4]
                })
            },
    
            MidiMetaEventType::TimeSignature => {
                let mut bytes: [u8; 4] = [0; 4];
                read_with_eof_check!(midi_stream, &mut bytes);
                Ok(MidiMetaEvent::TimeSignature {
                    numerator: bytes[0],
                    denominator: bytes[1],
                    metro: bytes[2],
                    thirty_seconds: bytes[3]
                })
            },
    
            MidiMetaEventType::KeySignature => {
                let mut bytes: [u8; 2] = [0; 2];
                read_with_eof_check!(midi_stream, &mut bytes);
                Ok(MidiMetaEvent::KeySignature {
                    key: bytes[0],
                    scale: bytes[1]
                })
            }
    
            MidiMetaEventType::SequencerSpecific => {
                // WRONG. This is not variable length. it's just a bunch of bytes until size is fulfilled
                let data = match parse_variable_length(midi_stream) {
                    Ok(data) => data,
                    Err(err) => {
                        let msg = format!("Failed to sequencer specific meta event: {}", err);
                        return Err(MidiError::new(&msg));
                    }
                };
                Ok(MidiMetaEvent::SequencerSpecific{ data })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sequence_number() {
        let expected_event = MidiMetaEvent::SequenceNumber{ msb: 32, lsb: 64 };
        let type_byte = MidiMetaEventType::SequenceNumber.to_byte();
        let mut bytes: &[u8] = &[type_byte, 2, 32, 64];
        let event = MidiMetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_text_event() {
        let expected_event = MidiMetaEvent::TextEvent{ text: "text".to_string() };
        let type_byte = MidiMetaEventType::TextEvent.to_byte();
        let mut bytes: &[u8] = &[type_byte, 4, 't' as u8, 'e' as u8, 'x' as u8, 't' as u8];
        let event = MidiMetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_copyright_notice() {
        let expected_event = MidiMetaEvent::CopyrightNotice{ text: "text".to_string() };
        let type_byte = MidiMetaEventType::CopyrightNotice.to_byte();
        let mut bytes: &[u8] = &[type_byte, 4, 't' as u8, 'e' as u8, 'x' as u8, 't' as u8];
        let event = MidiMetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_sequence_or_track_name() {
        let expected_event = MidiMetaEvent::SequenceOrTrackName{ text: "text".to_string() };
        let type_byte = MidiMetaEventType::SequenceOrTrackName.to_byte();
        let mut bytes: &[u8] = &[type_byte, 4, 't' as u8, 'e' as u8, 'x' as u8, 't' as u8];
        let event = MidiMetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_instrument_name() {
        let expected_event = MidiMetaEvent::InstrumentName{ text: "text".to_string() };
        let type_byte = MidiMetaEventType::InstrumentName.to_byte();
        let mut bytes: &[u8] = &[type_byte, 4, 't' as u8, 'e' as u8, 'x' as u8, 't' as u8];
        let event = MidiMetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_lyrics() {
        let expected_event = MidiMetaEvent::Lyrics{ text: "text".to_string() };
        let type_byte = MidiMetaEventType::Lyrics.to_byte();
        let mut bytes: &[u8] = &[type_byte, 4, 't' as u8, 'e' as u8, 'x' as u8, 't' as u8];
        let event = MidiMetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_marker() {
        let expected_event = MidiMetaEvent::Marker{ text: "text".to_string() };
        let type_byte = MidiMetaEventType::Marker.to_byte();
        let mut bytes: &[u8] = &[type_byte, 4, 't' as u8, 'e' as u8, 'x' as u8, 't' as u8];
        let event = MidiMetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_cue_point() {
        let expected_event = MidiMetaEvent::CuePoint{ text: "text".to_string() };
        let type_byte = MidiMetaEventType::CuePoint.to_byte();
        let mut bytes: &[u8] = &[type_byte, 4, 't' as u8, 'e' as u8, 'x' as u8, 't' as u8];
        let event = MidiMetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_midi_channel_prefix() {
        let expected_event = MidiMetaEvent::MidiChannelPrefix{ channel: 42 };
        let type_byte = MidiMetaEventType::MidiChannelPrefix.to_byte();
        let mut bytes: &[u8] = &[type_byte, 1, 42];
        let event = MidiMetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_end_of_track() {
        let expected_event = MidiMetaEvent::EndOfTrack;
        let type_byte = MidiMetaEventType::EndOfTrack.to_byte();
        let mut bytes: &[u8] = &[type_byte, 0];
        let event = MidiMetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_set_tempo() {
        let expected_event = MidiMetaEvent::SetTempo{ tempo: 42 };
        let type_byte = MidiMetaEventType::SetTempo.to_byte();
        let mut bytes: &[u8] = &[type_byte, 4, 0, 0, 0, 42];
        let event = MidiMetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_smpte_offset() {
        let expected_event = MidiMetaEvent::SmpteOffset{ 
            hour: 1,
            min: 2,
            sec: 3,
            frame: 4,
            sub_frame: 5
        };
        let type_byte = MidiMetaEventType::SmpteOffset.to_byte();
        let mut bytes: &[u8] = &[type_byte, 5, 1, 2, 3, 4, 5];
        let event = MidiMetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_time_signature() {
        let expected_event = MidiMetaEvent::TimeSignature{ 
            numerator: 1,
            denominator: 2,
            metro: 3,
            thirty_seconds: 4,
        };
        let type_byte = MidiMetaEventType::TimeSignature.to_byte();
        let mut bytes: &[u8] = &[type_byte, 4, 1, 2, 3, 4];
        let event = MidiMetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_key_signature() {
        let expected_event = MidiMetaEvent::KeySignature{ key: 1, scale: 2 };
        let type_byte = MidiMetaEventType::KeySignature.to_byte();
        let mut bytes: &[u8] = &[type_byte, 2, 1, 2];
        let event = MidiMetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }

    #[test]
    fn parse_sequencer_specific() {
        let expected_event = MidiMetaEvent::SequencerSpecific{ data: 42 };
        let type_byte = MidiMetaEventType::SequencerSpecific.to_byte();
        let mut bytes: &[u8] = &[type_byte, 1, 42];
        let event = MidiMetaEvent::parse(&mut bytes).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event does not mach expected");
    }
}