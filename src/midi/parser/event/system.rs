use std::io;

use super::super::{parse_variable_length};
use crate::midi::error::*;

const NORMAL_TYPE_BYTE: u8 = 0xF0;
const DIVIDED_OR_AUTH_TYPE_BYTE: u8 = 0xF7;

#[derive(PartialEq, Debug, Clone)]
pub enum MidiSystemEvent {
    Normal(Vec<u8>),
    Divided(Vec<u8>),
    Authorization(Vec<u8>)
}

impl MidiSystemEvent {
    pub fn parse<T: io::Read>(
        mut midi_stream: T,
        type_byte: u8,
        divided_bytes: &mut Vec<u8>
    ) -> MidiResult<MidiSystemEvent> {
        let size = match parse_variable_length(&mut midi_stream) {
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
            NORMAL_TYPE_BYTE => {
                // Normal system event or beginning of divided event
                let mut data = Vec::with_capacity(size);
                data.resize(size, 0_u8);
                let data_slice = &mut data;
                read_with_eof_check!(midi_stream, data_slice);
                if *data.last().expect("Data is empty? But size isn't 0?") == 0xF7 {
                    data.pop(); // remove the 0xF7 byte that indicated the end
                    return Ok(MidiSystemEvent::Normal(data));
                }
                // There's more coming later that we'll need to append to this
                divided_bytes.clear();
                divided_bytes.extend_from_slice(&data);
                Ok(MidiSystemEvent::Divided(data))
            },

            DIVIDED_OR_AUTH_TYPE_BYTE => {
                // We're in the middle of a divided event, or this is an Authorization event
                if !divided_bytes.is_empty() {
                    let mut additional_data = Vec::with_capacity(size);
                    additional_data.resize(size, 0_u8);
                    let data_slice = &mut additional_data;
                    read_with_eof_check!(midi_stream, data_slice);
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
                    read_with_eof_check!(midi_stream, data_slice);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_normal() {
        let expected_event: MidiSystemEvent = MidiSystemEvent::Normal(vec![42, 42, 42]);
        let mut event_data: &[u8] = &[4, 42, 42, 42, 0xF7];
        let mut divided_bytes = Vec::new();
        let event = MidiSystemEvent::parse(
            &mut event_data, NORMAL_TYPE_BYTE, &mut divided_bytes
        ).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event did not match expected");
    }

    #[test]
    fn parse_divided() {
        let expected_first_event: MidiSystemEvent = MidiSystemEvent::Divided(vec![42]);
        let expected_second_event: MidiSystemEvent = MidiSystemEvent::Divided(vec![42, 42]);
        let expected_final_event: MidiSystemEvent = MidiSystemEvent::Normal(vec![42, 42, 42]);
        let mut divided_bytes = Vec::new();

        let mut event_data: &[u8] = &[1, 42];
        let event = MidiSystemEvent::parse(
            &mut event_data, NORMAL_TYPE_BYTE, &mut divided_bytes
        ).expect("Failed to parse");
        assert_eq!(event, expected_first_event, "First event did not match expected");

        let mut event_data: &[u8] = &[1, 42];
        let event = MidiSystemEvent::parse(
            &mut event_data, DIVIDED_OR_AUTH_TYPE_BYTE, &mut divided_bytes
        ).expect("Failed to parse");
        assert_eq!(event, expected_second_event, "Second event did not match expected");

        let mut event_data: &[u8] = &[2, 42, 0xF7];
        let event = MidiSystemEvent::parse(
            &mut event_data, DIVIDED_OR_AUTH_TYPE_BYTE, &mut divided_bytes
        ).expect("Failed to parse");
        assert_eq!(event, expected_final_event, "Final event did not match expected");
    }

    #[test]
    fn parse_auth() {
        let expected_event: MidiSystemEvent = MidiSystemEvent::Authorization(vec![42, 42, 42]);
        let mut event_data: &[u8] = &[3, 42, 42, 42];
        let mut divided_bytes = Vec::new();
        let event = MidiSystemEvent::parse(
            &mut event_data, DIVIDED_OR_AUTH_TYPE_BYTE, &mut divided_bytes
        ).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event did not match expected");
    }
}