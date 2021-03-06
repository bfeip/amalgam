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
}

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
    pub fn parse<T: io::Read>(midi_file_stream: &mut T) -> MidiResult<MidiMetaEvent> {
        let mut meta_event_type_byte: [u8; 1] = [0; 1];
        read_with_eof_check!(midi_file_stream, &mut meta_event_type_byte);
        
        let size = match parse_variable_length(midi_file_stream) {
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
                read_with_eof_check!(midi_file_stream, &mut bytes);
                Ok(MidiMetaEvent::SequenceNumber{ msb: bytes[0], lsb: bytes[1] })
            },
    
            MidiMetaEventType::TextEvent => {
                let text = match parse_string(midi_file_stream, size) {
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
                let text = match parse_string(midi_file_stream, size) {
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
                let text = match parse_string(midi_file_stream, size) {
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
                let text = match parse_string(midi_file_stream, size) {
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
                let text = match parse_string(midi_file_stream, size) {
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
                let text = match parse_string(midi_file_stream, size) {
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
                let text = match parse_string(midi_file_stream, size) {
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
                read_with_eof_check!(midi_file_stream, &mut channel_byte);
                Ok(MidiMetaEvent::MidiChannelPrefix{ channel: channel_byte[0] })
            },
    
            MidiMetaEventType::EndOfTrack => Ok(MidiMetaEvent::EndOfTrack),
    
            MidiMetaEventType::SetTempo => {
                let mut tempo_bytes: [u8; 4] = [0; 4];
                read_with_eof_check!(midi_file_stream, &mut tempo_bytes);
                let tempo = u32::from_be_bytes(tempo_bytes);
                Ok(MidiMetaEvent::SetTempo{ tempo })
            },
    
            MidiMetaEventType::SmpteOffset => {
                let mut bytes: [u8; 5] = [0; 5];
                read_with_eof_check!(midi_file_stream, &mut bytes);
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
                read_with_eof_check!(midi_file_stream, &mut bytes);
                Ok(MidiMetaEvent::TimeSignature {
                    numerator: bytes[0],
                    denominator: bytes[1],
                    metro: bytes[2],
                    thirty_seconds: bytes[3]
                })
            },
    
            MidiMetaEventType::KeySignature => {
                let mut bytes: [u8; 2] = [0; 2];
                read_with_eof_check!(midi_file_stream, &mut bytes);
                Ok(MidiMetaEvent::KeySignature {
                    key: bytes[0],
                    scale: bytes[1]
                })
            }
    
            MidiMetaEventType::SequencerSpecific => {
                // WRONG. This is not variable length. it's just a bunch of bytes until size is fulfilled
                let data = match parse_variable_length(midi_file_stream) {
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