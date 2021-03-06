use std::io;

use super::super::{error::*, parse_variable_length};

pub enum MidiSystemEvent {
    Normal(Vec<u8>),
    Divided(Vec<u8>),
    Authorization(Vec<u8>)
}

impl MidiSystemEvent {
    // Might return a MIDI system event or nothing if the system event is divided and not yet complete
    pub fn parse<T: io::Read>(
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
                let msg = format!("Tried to parse system byte but type was unexpected {:#04x}", type_byte);
                return Err(MidiError::new(&msg));
            }
        }
    }
}