use std::io;

use super::super::{parse_variable_length};
use crate::{SynthError, SynthResult};

const NORMAL_TYPE_BYTE: u8 = 0xF0;
const DIVIDED_OR_AUTH_TYPE_BYTE: u8 = 0xF7;

#[derive(PartialEq, Debug, Clone)]
pub enum SystemEvent {
    Normal(Vec<u8>),
    Divided(Vec<u8>),
    Authorization(Vec<u8>)
}

impl SystemEvent {
    pub fn parse<T: io::Read>(
        mut midi_stream: T,
        type_byte: u8,
        divided_bytes: &mut Vec<u8>
    ) -> SynthResult<SystemEvent> {
        let size = match parse_variable_length(&mut midi_stream) {
            Ok(size) => size,
            Err(err) => {
                let msg = format!("Failed to parse system event size: {}", err);
                return Err(SynthError::new(&msg));
            }
        };
        if size == 0 {
            let msg = "Size of system event is 0";
            return Err(SynthError::new(msg));
        }

        match type_byte {
            NORMAL_TYPE_BYTE => {
                // Normal system event or beginning of divided event
                let mut data = vec![0; size];
                let data_slice = &mut data;
                read_with_eof_check!(midi_stream, data_slice);
                if *data.last().expect("Data is empty? But size isn't 0?") == 0xF7 {
                    data.pop(); // remove the 0xF7 byte that indicated the end
                    return Ok(SystemEvent::Normal(data));
                }
                // There's more coming later that we'll need to append to this
                divided_bytes.clear();
                divided_bytes.extend_from_slice(&data);
                Ok(SystemEvent::Divided(data))
            },

            DIVIDED_OR_AUTH_TYPE_BYTE => {
                // We're in the middle of a divided event, or this is an Authorization event
                if !divided_bytes.is_empty() {
                    let mut additional_data = vec![0; size];
                    let data_slice = &mut additional_data;
                    read_with_eof_check!(midi_stream, data_slice);
                    divided_bytes.extend_from_slice(data_slice);
                    if *divided_bytes.last().expect("Static data is empty?") == 0xF7 {
                        divided_bytes.pop(); // remove the 0xF7 byte that indicated the end
                        return Ok(SystemEvent::Normal(divided_bytes.clone()));
                    }
                    // There's even more
                    // TODO: We're passed a reference for divided bytes that we extend and then clone here.
                    // This is inefficient since the bytes in a divided event aren't even useful until
                    // the whole thing is completed.
                    Ok(SystemEvent::Divided(divided_bytes.clone()))
                }
                else {
                    // Authorization event
                    let mut event_data = vec![0; size];
                    let data_slice = &mut event_data;
                    read_with_eof_check!(midi_stream, data_slice);
                    Ok(SystemEvent::Authorization(event_data))
                }
            }
            _ => {
                let msg = format!("Tried to parse system byte but type was unexpected {:#04x}", type_byte);
                Err(SynthError::new(&msg))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_normal() {
        let expected_event: SystemEvent = SystemEvent::Normal(vec![42, 42, 42]);
        let mut event_data: &[u8] = &[4, 42, 42, 42, 0xF7];
        let mut divided_bytes = Vec::new();
        let event = SystemEvent::parse(
            &mut event_data, NORMAL_TYPE_BYTE, &mut divided_bytes
        ).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event did not match expected");
    }

    #[test]
    fn parse_divided() {
        let expected_first_event: SystemEvent = SystemEvent::Divided(vec![42]);
        let expected_second_event: SystemEvent = SystemEvent::Divided(vec![42, 42]);
        let expected_final_event: SystemEvent = SystemEvent::Normal(vec![42, 42, 42]);
        let mut divided_bytes = Vec::new();

        let mut event_data: &[u8] = &[1, 42];
        let event = SystemEvent::parse(
            &mut event_data, NORMAL_TYPE_BYTE, &mut divided_bytes
        ).expect("Failed to parse");
        assert_eq!(event, expected_first_event, "First event did not match expected");

        let mut event_data: &[u8] = &[1, 42];
        let event = SystemEvent::parse(
            &mut event_data, DIVIDED_OR_AUTH_TYPE_BYTE, &mut divided_bytes
        ).expect("Failed to parse");
        assert_eq!(event, expected_second_event, "Second event did not match expected");

        let mut event_data: &[u8] = &[2, 42, 0xF7];
        let event = SystemEvent::parse(
            &mut event_data, DIVIDED_OR_AUTH_TYPE_BYTE, &mut divided_bytes
        ).expect("Failed to parse");
        assert_eq!(event, expected_final_event, "Final event did not match expected");
    }

    #[test]
    fn parse_auth() {
        let expected_event: SystemEvent = SystemEvent::Authorization(vec![42, 42, 42]);
        let mut event_data: &[u8] = &[3, 42, 42, 42];
        let mut divided_bytes = Vec::new();
        let event = SystemEvent::parse(
            &mut event_data, DIVIDED_OR_AUTH_TYPE_BYTE, &mut divided_bytes
        ).expect("Failed to parse");
        assert_eq!(event, expected_event, "Event did not match expected");
    }
}