// Info about MIDI used to write this:
// https://web.archive.org/web/20141227205754/http://www.sonicspot.com:80/guide/midifiles.html
#![macro_use]

use super::error::*;
use event::Event;

macro_rules! read_with_eof_check {
    ($midiFileStream:expr, $buffer:expr) => {
        if let Err(err) = $midiFileStream.read_exact($buffer) {
            let msg = format!("Unexpected EOF: {}", err);
            return Err(MidiError::new(&msg));
        }
    };
}

pub mod event;

use std::io;
use std::fs::File;

#[derive(Debug)]
pub struct MidiData {
    header: HeaderChunk,
    tracks: Vec<TrackChunk>
}

impl MidiData {
    fn new(header: HeaderChunk, tracks: Vec<TrackChunk>) -> Self {
        Self { header, tracks }
    }

    pub fn from_file<P: AsRef<std::path::Path>>(midi_path: P) -> MidiResult<Self> {
        let midi_file = match File::open(&midi_path) {
            Ok(midi_file) => midi_file,
            Err(err) => {
                let msg = format!("Failed to open MIDI file at {} for reading: {}", midi_path.as_ref().display(), err);
                return Err(MidiError::new(&msg));
            }
        };
        let mut midi_file_stream = io::BufReader::new(midi_file);
    
        let header = match HeaderChunk::parse(&mut midi_file_stream) {
            Ok(header) => header,
            Err(err) => {
                let msg = format!("Failed to parse MIDI header: {}", err);
                return Err(MidiError::new(&msg));
            }
        };
        let mut tracks = Vec::with_capacity(header.n_tracks);
        for _ in 0..header.n_tracks {
            let track = match TrackChunk::parse(&mut midi_file_stream) {
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

    pub fn iter_tracks(&self) -> std::slice::Iter<TrackChunk> {
        self.tracks.iter()
    }

    pub fn get_track_count(&self) -> usize {
        self.header.n_tracks
    }

    pub fn get_header(&self) -> &HeaderChunk {
        &self.header
    }
}

#[derive(Debug, Copy, Clone)]
pub enum TimeDivision {
    TicksPerBeat(u16),
    FramesPerSecond{ frames_per_second: u8, ticks_per_frame: u8}
}

#[derive(Debug)]
enum Format {
    UniTrack = 0,
    MultiTrack = 1,
    MultiUniTrack = 2
}

#[derive(Debug)]
pub struct HeaderChunk {
    format: Format, // 2 bytes in file
    n_tracks: usize, // 2 bytes in file
    time_division: TimeDivision
}

impl HeaderChunk {
    fn new(format: Format, n_tracks: usize, time_division: TimeDivision) -> Self {
        HeaderChunk { format, n_tracks, time_division }
    }

    fn from_bytes(format_bytes: [u8; 2], n_tracks_bytes: [u8; 2], time_division_bytes: [u8; 2]) -> MidiResult<Self> {
        let format = match format_bytes[1] {
            0 => Format::UniTrack,
            1 => Format::MultiTrack,
            2 => Format::MultiUniTrack,
            _ => {
                let format_u16 = u16::from_be_bytes(format_bytes);
                let msg = format!("Got unknown MIDI format {:#06x}", format_u16);
                return Err(MidiError::new(&msg));
            }
        };

        let n_tracks = u16::from_be_bytes(n_tracks_bytes) as usize;

        let time_division_u16 = u16::from_be_bytes(time_division_bytes);
        let time_division = match time_division_u16 & 0x8000 {
            0x8000 => {
                let frames_per_second = (time_division_u16 >> 8) as u8 & 0x7F as u8;
                let ticks_per_frame = (time_division_u16 & 0xFF) as u8;
                TimeDivision::FramesPerSecond { frames_per_second, ticks_per_frame}
            },
            0x0000 => TimeDivision::TicksPerBeat(time_division_u16),
            _ => {
                let msg = format!("Unknown time division {:#06x}", time_division_u16);
                return Err(MidiError::new(&msg));
            }
        };

        Ok(HeaderChunk { format, n_tracks, time_division })
    }

    fn parse<T: io::Read>(mut midi_stream: T) -> MidiResult<Self> {
        const EXPECTED_ID: &[u8] = "MThd".as_bytes();
        const EXPECTED_SIZE: u32 = 6;
    
        let mut id: [u8; 4] = [0; 4];
        let mut size: [u8; 4] = [0; 4];
        let mut format: [u8; 2] = [0; 2];
        let mut n_tracks: [u8; 2] = [0; 2];
        let mut time_division: [u8; 2] = [0; 2];
    
        // We only need to check that the ID is what we expected... It's useless after that
        read_with_eof_check!(midi_stream, &mut id);
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
        read_with_eof_check!(midi_stream, &mut size);
        let size_u32 = u32::from_be_bytes(size);
        if size_u32 != EXPECTED_SIZE {
            let msg = format!("Expected main header size to be {}. Got {}", EXPECTED_SIZE, size_u32);
            return Err(MidiError::new(&msg)); 
        }
    
        read_with_eof_check!(midi_stream, &mut format);
        read_with_eof_check!(midi_stream, &mut n_tracks);
        read_with_eof_check!(midi_stream, &mut time_division);
    
        let header_chunk = Self::from_bytes(format, n_tracks, time_division);
        #[cfg(feature = "verbose_midi")]
        {
            println!("Read MIDI header chunk:\n{:#?}", header_chunk);
        }
        header_chunk
    }

    pub fn get_time_division(&self) -> TimeDivision {
        self.time_division
    }
}

#[derive(Debug)]
pub struct TrackChunk {
    events: Vec<Event>,
}

impl TrackChunk {
    fn new(events: Vec<Event>) -> Self {
        TrackChunk { events }
    }

    fn parse<T: io::Read + io::Seek>(mut midi_stream: T) -> MidiResult<Self> {
        const EXPECTED_ID: &[u8] = "MTrk".as_bytes();
        
        let mut id_bytes: [u8; 4] = [0; 4];
        let mut size_bytes: [u8; 4]= [0; 4];
    
        // We only need to check that the ID is what we expected... It's useless after that
        read_with_eof_check!(midi_stream, &mut id_bytes);
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
        #[cfg(feature = "verbose_midi")]
        {
            println!("Parsed track id and it matches the expected ID: {:?}", EXPECTED_ID);
        }
    
        read_with_eof_check!(midi_stream, &mut size_bytes);
        let size = u32::from_be_bytes(size_bytes) as u64;
        #[cfg(feature = "verbose_midi")]
        {
            println!("Parsed track size: {}", size);
        }
    
        let divided_event_bytes: &mut Vec<u8> = &mut Vec::new();
        let mut events = Vec::new();
        const HERE: io::SeekFrom = io::SeekFrom::Current(0);
        let start_stream_position = midi_stream.seek(HERE).expect("Failed to get stream position");
        while midi_stream.seek(HERE).unwrap() - start_stream_position < size {
            // While we haven't met the size yet
            #[cfg(feature = "verbose_midi")]
            {
                let stream_position = midi_stream.seek(HERE).unwrap();
                println!("Current MIDI stream position: {}", stream_position);
            }
            let event = match Event::parse(&mut midi_stream, divided_event_bytes) {
                Ok(event) => event,
                Err(err) => {
                    let msg = format!("Failed to parse events: {}", err);
                    return Err(MidiError::new(&msg));
                }
            };
            #[cfg(feature = "verbose_midi")]
            {
                println!("Parsed event:\n{:#?}", event)
            }
            events.push(event)
        }
        if midi_stream.seek(HERE).unwrap() - start_stream_position > size {
            let msg = "Read more than size of track";
            return Err(MidiError::new(msg));
        }
        let track_chunk = TrackChunk::new(events);
        #[cfg(feature = "verbose_midi")]
        {
            println!("Read MIDI track chunk:\n{:#?}", track_chunk)
        }
        Ok(track_chunk)
    }

    pub fn iter_events(&self) -> std::slice::Iter<Event> {
        self.events.iter()
    }
}

// TODO: This needs many tests
fn parse_variable_length<T: io::Read>(mut midi_stream: T) -> MidiResult<usize> {
    let mut byte = [0x80_u8; 1];
    let mut bytes = Vec::new();
    while byte[0] & 0x80 == 0x80 {
        // a leading 1 on a byte indicates that there is a byte that follows
        read_with_eof_check!(midi_stream, &mut byte);
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

    #[cfg(feature = "verbose_midi")]
    {
        println!("Parsed MIDI variable langth field with bytes {:?} and value {:?}", bytes, total_value);
    }
    Ok(total_value)
}

fn parse_string<T: io::Read>(mut midi_stream: T, size: usize) -> MidiResult<String> {
    let mut byte_array = Vec::with_capacity(size);
    byte_array.resize(size, 0_u8);
    read_with_eof_check!(midi_stream, &mut byte_array);
    let string = match String::from_utf8(byte_array) {
        Ok(string) => string,
        Err(err) => {
            let msg = format!("Failed to parse string with size {}: {}", size, err);
            return Err(MidiError::new(&msg));
        }
    };
    #[cfg(feature = "verbose_midi")]
    {
        println!("Parsed MIDI string: {}", string);
    }
    Ok(string)
}

fn throw_unexpected_eof(err: io::Error) -> MidiResult<()> {
    let msg = format!("Unexpected EOF: {}", err);
    return Err(MidiError::new(&msg));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::{env, fs};

    fn get_repo_root() -> PathBuf {
        let mut cur_dir = env::current_dir().expect("Couldn't get working dir?");
        loop {
            let contents = match fs::read_dir(&cur_dir) {
                Ok(contents) => contents,
                Err(_err) => {
                    panic!("Failed to read contents of {}", cur_dir.display());
                }
            };
            for dir_item in contents {
                if dir_item.is_err() {
                    continue;
                }
                let item_name = dir_item.unwrap().file_name();
                if item_name.to_str().unwrap() == "Cargo.toml" {
                    return cur_dir;
                }
            }
            if cur_dir.pop() == false {
                panic!("Failed to find repo root");
            }
        }
    }

    fn get_test_midi_file_path() -> PathBuf {
        let test_midi_file_path_from_root: PathBuf = ["data", "never_gonna_give_you_up.mid"].iter().collect();
        let repo_root = get_repo_root();
        repo_root.join(test_midi_file_path_from_root)
    }

    #[test]
    fn read_midi_file() {
        let midi_file_path_buf = get_test_midi_file_path();
        let midi_file_path_str = midi_file_path_buf.as_os_str().to_str().expect(
            "Couldn't get str of test midi path"
        );
        if let Err(err) = MidiData::from_file(midi_file_path_str) {
            panic!("Failed to parse MIDI file: {}", err);
        }
    }
}