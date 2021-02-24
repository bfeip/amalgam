// Info about MIDI used to write this:
// https://web.archive.org/web/20141227205754/http://www.sonicspot.com:80/guide/midifiles.html

use std::io;
use std::fs::File;

#[derive(Debug)]
pub struct MidiError {
    msg: String
}

impl MidiError {
    pub fn new(msg: &str) -> Self {
        let msg = msg.to_string();
        MidiError { msg }
    }
}

impl std::fmt::Display for MidiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    } 
}

impl std::error::Error for MidiError {}

pub type MidiResult<T> = Result<T, MidiError>;

struct MidiData {
    header: HeaderChunk,
    tracks: Vec<TrackChunk>
}

impl MidiData {
    fn new(header: HeaderChunk, tracks: Vec<TrackChunk>) -> Self {
        Self { header, tracks }
    }
}

enum TimeDivision {
    TicksPerBeat(u16),
    FramesPerSecond(u16)
}

enum MidiFormat {
    UniTrack = 0,
    MultiTrack = 1,
    MultiUniTrack = 2
}

struct HeaderChunk {
    format: MidiFormat, // 2 bytes in file
    n_tracks: usize, // 2 bytes in file
    time_division: TimeDivision
}

impl HeaderChunk {
    fn new(format: MidiFormat, n_tracks: usize, time_division: TimeDivision) -> Self {
        HeaderChunk { format, n_tracks, time_division }
    }

    fn from_bytes(format_bytes: [u8; 2], n_tracks_bytes: [u8; 2], time_division_bytes: [u8; 2]) -> MidiResult<Self> {
        let format = match format_bytes[1] {
            0 => MidiFormat::UniTrack,
            1 => MidiFormat::MultiTrack,
            2 => MidiFormat::MultiUniTrack,
            _ => {
                let format_u16 = u16::from_be_bytes(format_bytes);
                let msg = format!("Got unknown MIDI format {:#06x}", format_u16);
                return Err(MidiError::new(&msg));
            }
        };

        let n_tracks = u16::from_be_bytes(n_tracks_bytes) as usize;

        let time_division_u16 = u16::from_be_bytes(time_division_bytes);
        let time_division = match time_division_u16 & 0x8000 {
            0x8000 => TimeDivision::FramesPerSecond(time_division_u16 & !0x8000),
            0x0000 => TimeDivision::TicksPerBeat(time_division_u16),
            _ => {
                let msg = format!("Unknown time division {:#06x}", time_division_u16);
                return Err(MidiError::new(&msg));
            }
        };

        Ok(HeaderChunk { format, n_tracks, time_division })
    }
}

struct TrackChunk {
    events: Vec<MidiEvent>
}

impl TrackChunk {
    fn new(events: Vec<MidiEvent>) -> Self {
        TrackChunk { events }
    }
}

enum MidiEventType {
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
    fn from_nybble(nybble: u8) -> MidiResult<Self> {
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

struct MidiChannelEvent {
    delta_time: usize, // variable length in file
    event_type: MidiEventType, // 4 bits in file
    channel: u8, // 4 bits in file
    param1: u8,
    param2: u8
}

impl MidiChannelEvent {
    fn new(delta_time: usize, event_type: MidiEventType, channel: u8, param1: u8, param2: u8) -> Self {
        Self { delta_time, event_type, channel, param1, param2 }
    }
}

enum MidiEvent {
    Channel(MidiChannelEvent),
    Meta(MidiMetaEvent),
    System(MidiSystemEvent)
}

enum MidiControllerEvent {
    BankSelect,
    Modulation,
    BreathController,
    FootController,
    PortamentoTime,
    DataEntryMsb,
    MainVolume,
    Balance,
    Pan,
    ExpressionController,
    EffectControl1,
    EffectControl2,
    GeneralPurposeControllers(u8), 
    LsbForControllers(u8),
    DamperPedal, // sustain
    Portamento,
    Sostenuto,
    SoftPedal,
    LegatoFootswitch,
    Hold2,
    SoundController(u8), // (1: Timber Variation, 2: Timber/Harmonic Content 3: Release, 4: Attack)
    PortamentoControl,
    EffectsDepth(u8), // (formerly External Effects Depth)
    DataIncrement,
    DataDecrement,
    NonRegisteredParameterNumberLsb,
    NonRegisteredParameterNumberMsb,
    RegisteredParameterNumberLsb,
    RegisteredParameterNumberMsb,
    ModeMessages(u8)
}

impl MidiControllerEvent {
    fn from_byte(byte: u8) -> MidiResult<Self> {
        let controller_event = match byte {
            0x00        => MidiControllerEvent::BankSelect,
            0x01        => MidiControllerEvent::Modulation,
            0x02        => MidiControllerEvent::BreathController,
            0x04        => MidiControllerEvent::FootController,
            0x05        => MidiControllerEvent::PortamentoTime,
            0x06        => MidiControllerEvent::DataEntryMsb,
            0x07        => MidiControllerEvent::MainVolume,
            0x08        => MidiControllerEvent::Balance,
            0x0A        => MidiControllerEvent::Pan,
            0x0B        => MidiControllerEvent::ExpressionController,
            0x0C        => MidiControllerEvent::EffectControl1,
            0x0D        => MidiControllerEvent::EffectControl2,
            0x10..=0x13 => MidiControllerEvent::GeneralPurposeControllers(byte - 0x10),
            0x20..=0x3F => MidiControllerEvent::LsbForControllers(byte - 0x20),
            0x40        => MidiControllerEvent::DamperPedal,
            0x41        => MidiControllerEvent::Portamento,
            0x42        => MidiControllerEvent::Sostenuto,
            0x43        => MidiControllerEvent::SoftPedal,
            0x44        => MidiControllerEvent::LegatoFootswitch,
            0x45        => MidiControllerEvent::Hold2,
            0x46..=0x4F => MidiControllerEvent::SoundController(byte - 0x46),
            0x50..=0x53 => MidiControllerEvent::GeneralPurposeControllers(byte - 0x50),
            0x54        => MidiControllerEvent::PortamentoControl,
            0x5B..=0x5F => MidiControllerEvent::EffectsDepth(byte - 0x5B),
            0x60        => MidiControllerEvent::DataIncrement,
            0x61        => MidiControllerEvent::DataDecrement,
            0x62        => MidiControllerEvent::NonRegisteredParameterNumberLsb,
            0x63        => MidiControllerEvent::NonRegisteredParameterNumberMsb,
            0x64        => MidiControllerEvent::RegisteredParameterNumberLsb,
            0x65        => MidiControllerEvent::RegisteredParameterNumberMsb,
            0x79..=0x7F => MidiControllerEvent::ModeMessages(byte - 0x79),
            _ => {
                let msg = format!("unknown MIDI controller message {:#04x}", byte);
                return Err(MidiError::new(&msg)) 
            }
        };
        Ok(controller_event)
    }
}

enum MidiMetaEventType {
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
    fn from_byte(byte: u8) -> MidiResult<Self> {
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

enum MidiMetaEvent {
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

enum MidiSystemEvent {
    Normal(Vec<u8>),
    Divided(Vec<u8>),
    Authorization(Vec<u8>)
}

macro_rules! read_with_eof_check {
    ($midiFileStream:expr, $buffer:expr) => {
        if let Err(err) = $midiFileStream.read_exact($buffer) {
            let msg = format!("Unexpected EOF: {}", err);
            return Err(MidiError::new(&msg));
        }
    };
}

fn parse_midi(midi_path: &str) -> MidiResult<MidiData> {
    let midi_file = match File::open(midi_path) {
        Ok(midi_file) => midi_file,
        Err(err) => {
            let msg = format!("Failed to open MIDI file at {} for reading: {}", midi_path, err);
            return Err(MidiError::new(&msg));
        }
    };
    let mut midi_file_stream = io::BufReader::new(midi_file);

    let header = match parse_header_chunk(&mut midi_file_stream) {
        Ok(header) => header,
        Err(err) => {
            let msg = format!("Failed to parse MIDI header: {}", err);
            return Err(MidiError::new(&msg));
        }
    };
    let mut tracks = Vec::with_capacity(header.n_tracks);
    for _ in 0..header.n_tracks {
        let track = match parse_track_chunk(&mut midi_file_stream) {
            Ok(track) => track,
            Err(err) => {
                let msg = format!("Failed to parse MIDI track: {}", err);
                return Err(MidiError::new(&msg));
            }
        };
        tracks.push(track);
    }
    Ok(MidiData::new(header, tracks))
}

fn parse_header_chunk<T: io::Read>(midi_file_stream: &mut T) -> MidiResult<HeaderChunk> {
    const EXPECTED_ID: &[u8] = "MThd".as_bytes();
    const EXPECTED_SIZE: u32 = 6;

    let mut id: [u8; 4] = [0; 4];
    let mut size: [u8; 4] = [0; 4];
    let mut format: [u8; 2] = [0; 2];
    let mut n_tracks: [u8; 2] = [0; 2];
    let mut time_division: [u8; 2] = [0; 2];

    // We only need to check that the ID is what we expected... It's useless after that
    read_with_eof_check!(midi_file_stream, &mut id);
    if !id.iter().eq(EXPECTED_ID.iter()) {
        let expected_id_str = std::str::from_utf8(EXPECTED_ID).expect("EXPECTED_ID was not valid UTF-8 somehow");
        let id_str = match std::str::from_utf8(&id) {
            Ok(id_str) => id_str,
            Err(err) => {
                let id_value = u32::from_be_bytes(id);
                let msg = format!(
                    "Expected main header ID to be {}. Got an invalid UTF-8 with value {:#010x}: {}",
                    expected_id_str, id_value, err
                );
                return Err(MidiError::new(&msg));
            }
        };
        let msg = format!("Expected main header ID to be {} got {}", expected_id_str, id_str);
        return Err(MidiError::new(&msg))
    }

    // The size of the main header should always be 6... Just check that it is and carry on
    read_with_eof_check!(midi_file_stream, &mut size);
    let size_u32 = u32::from_be_bytes(size);
    if size_u32 != EXPECTED_SIZE {
        let msg = format!("Expected main header size to be {}. Got {}", EXPECTED_SIZE, size_u32);
        return Err(MidiError::new(&msg)); 
    }

    read_with_eof_check!(midi_file_stream, &mut format);
    read_with_eof_check!(midi_file_stream, &mut n_tracks);
    read_with_eof_check!(midi_file_stream, &mut time_division);

    HeaderChunk::from_bytes(format, n_tracks, time_division)
}

fn parse_track_chunk<T: io::Read + io::Seek>(midi_file_stream: &mut T) -> MidiResult<TrackChunk> {
    const EXPECTED_ID: &[u8] = "MTrk".as_bytes();
    
    let mut id_bytes: [u8; 4] = [0; 4];
    let mut size_bytes: [u8; 4]= [0; 4];

    // We only need to check that the ID is what we expected... It's useless after that
    read_with_eof_check!(midi_file_stream, &mut id_bytes);
    if !id_bytes.iter().eq(EXPECTED_ID.iter()) {
        let expected_id_str = std::str::from_utf8(EXPECTED_ID).expect("EXPECTED_ID was not valid UTF-8 somehow");
        let id_str = match std::str::from_utf8(&id_bytes) {
            Ok(id_str) => id_str,
            Err(err) => {
                let id_value = u32::from_be_bytes(id_bytes);
                let msg = format!(
                    "Expected main header ID to be {}. Got an invalid UTF-8 with value {:#010x}: {}",
                    expected_id_str, id_value, err
                );
                return Err(MidiError::new(&msg));
            }
        };
        let msg = format!("Expected main header ID to be {} got {}", expected_id_str, id_str);
        return Err(MidiError::new(&msg));
    }

    read_with_eof_check!(midi_file_stream, &mut size_bytes);
    let size = u32::from_be_bytes(size_bytes) as u64;

    let divided_event_bytes: &mut Vec<u8> = &mut Vec::new();
    let mut events = Vec::new();
    const HERE: io::SeekFrom = io::SeekFrom::Current(0);
    let start_stream_position = midi_file_stream.seek(HERE).expect("Failed to get stream position");
    while midi_file_stream.seek(HERE).unwrap() - start_stream_position < size {
        // While we haven't met the size yet
        let event = match parse_event(midi_file_stream, divided_event_bytes) {
            Ok(event) => event,
            Err(err) => {
                let msg = format!("Failed to parse events: {}", err);
                return Err(MidiError::new(&msg));
            }
        };
        events.push(event)
    }
    if midi_file_stream.seek(HERE).unwrap() > size {
        let msg = "Read more than size of track";
        return Err(MidiError::new(msg));
    }
    Ok(TrackChunk::new(events))
}

fn parse_event<T: io::Read>(midi_file_stream: &mut T, divided_event_bytes: &mut Vec<u8>) -> MidiResult<MidiEvent> {
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
            parse_meta_or_system_event(midi_file_stream, event_type_and_channel_byte[0], divided_event_bytes)
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
            match parse_meta_event(midi_file_stream) {
                Ok(meta_event) => return Ok(MidiEvent::Meta(meta_event)),
                Err(err) => {
                    let msg = format!("Failed to parse meta event: {}", err);
                    return Err(MidiError::new(&msg));
                }
            };
        },
        0xF7 | 0xF0 => {
            match parse_system_event(midi_file_stream, event_type_and_channel_byte, divided_event_bytes) {
                Ok(system_event) => return Ok(MidiEvent::System(system_event)),
                Err(err) => {
                    let msg = format!("Failed to parse system event: {}", err);
                    return Err(MidiError::new(&msg));
                }
            };
        }
        _ => {
            let msg = format!(
                "Tried to parse meta or system event but got unknown type byte {:#04x}", event_type_and_channel_byte
            );
            return Err(MidiError::new(&msg));
        }
    }
}

fn parse_meta_event<T: io::Read>(midi_file_stream: &mut T) -> MidiResult<MidiMetaEvent> {
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

// Might return a MIDI system event or nothing if the system event is divided and not yet complete
fn parse_system_event<T: io::Read>(
    midi_file_stream: &mut T,
    type_byte: u8,
    divided_bytes: &mut Vec<u8>
) -> MidiResult<MidiSystemEvent> {
    let size = match parse_variable_length(midi_file_stream) {
        Ok(size) => size,
        Err(err) => {
            let msg = format!("Failed to parse system event size: {}", err);
            return Err(MidiError::new(&msg));
        }
    };
    if size == 0 {
        let msg = "Size of system event is 0";
        return Err(MidiError::new(msg));
    }

    match type_byte {
        0xF0 => {
            // Normal system event or beginning of divided event
            let mut data = Vec::with_capacity(size);
            data.resize(size, 0_u8);
            let data_slice = &mut data;
            read_with_eof_check!(midi_file_stream, data_slice);
            if *data.last().expect("Data is empty? But size isn't 0?") == 0xF7 {
                data.pop(); // remove the 0xF7 byte that indicated the end
                return Ok(MidiSystemEvent::Normal(data));
            }
            // There's more coming later that we'll need to append to this
            Ok(MidiSystemEvent::Divided(data))
        },

        0xF7 => {
            // We're in the middle of a divided event, or this is an Authorization event
            if !divided_bytes.is_empty() {
                let mut additional_data = Vec::with_capacity(size);
                additional_data.resize(size, 0_u8);
                let data_slice = &mut additional_data;
                read_with_eof_check!(midi_file_stream, data_slice);
                divided_bytes.extend_from_slice(data_slice);
                if *divided_bytes.last().expect("Static data is empty?") == 0xF7 {
                    divided_bytes.pop(); // remove the 0xF7 byte that indicated the end
                    return Ok(MidiSystemEvent::Normal(divided_bytes.clone()));
                }
                // There's even more
                // TODO: We're passed a reference for divided bytes that we extend and then clone here.
                // This is inefficent since the bytes in a divided event aren't even useful until
                // the whole thing is completed.
                return Ok(MidiSystemEvent::Divided(divided_bytes.clone()))
            }
            else {
                // Authorization event
                let mut event_data = Vec::with_capacity(size);
                event_data.resize(size, 0_u8);
                let data_slice = &mut event_data;
                read_with_eof_check!(midi_file_stream, data_slice);
                return Ok(MidiSystemEvent::Authorization(event_data))
            }
        }
        _ => {
            let msg = format!("Tried to parse system byte but type was unexpected {:04x}", type_byte);
            return Err(MidiError::new(&msg));
        }
    }
}

// TODO: This needs many tests
fn parse_variable_length<T: io::Read>(midi_file_stream: &mut T) -> MidiResult<usize> {
    let mut byte = [0x80_u8; 1];
    let mut bytes = Vec::new();
    while byte[0] & 0x80 == 0x80 {
        // a leading 1 on a byte indicates that there is a byte that follows
        read_with_eof_check!(midi_file_stream, &mut byte);
        bytes.push(byte[0]);
    }

    if bytes.len() > 7 {
        let msg = "Length of variable field exceeds what I expected";
        return Err(MidiError::new(msg));
    }

    let mut total_value = 0_usize;
    for i in (0..bytes.len()).rev() {
        let byte_value = (bytes[i] as usize & !0x80) << (7 * i);
        total_value += byte_value;
    }

    Ok(total_value)
}

fn parse_string<T: io::Read>(midi_file_stream: &mut T, size: usize) -> MidiResult<String> {
    let mut byte_array = Vec::with_capacity(size);
    byte_array.resize(size, 0_u8);
    read_with_eof_check!(midi_file_stream, &mut byte_array);
    let string = match String::from_utf8(byte_array) {
        Ok(string) => string,
        Err(err) => {
            let msg = format!("Failed to parse string with size {}: {}", size, err);
            return Err(MidiError::new(&msg));
        }
    };
    Ok(string)
}

fn throw_unexpected_eof(err: io::Error) -> MidiResult<()> {
    let msg = format!("Unexpected EOF: {}", err);
    return Err(MidiError::new(&msg));
}