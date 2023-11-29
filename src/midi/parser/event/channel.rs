use std::io;

use crate::{SynthError, SynthResult};
use super::EventType;

#[derive(Debug, Clone, PartialEq)]
pub enum ControllerEvent {
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

impl ControllerEvent {
    pub fn from_byte(byte: u8) -> SynthResult<Self> {
        let controller_event = match byte {
            0x00        => ControllerEvent::BankSelect,
            0x01        => ControllerEvent::Modulation,
            0x02        => ControllerEvent::BreathController,
            0x04        => ControllerEvent::FootController,
            0x05        => ControllerEvent::PortamentoTime,
            0x06        => ControllerEvent::DataEntryMsb,
            0x07        => ControllerEvent::MainVolume,
            0x08        => ControllerEvent::Balance,
            0x0A        => ControllerEvent::Pan,
            0x0B        => ControllerEvent::ExpressionController,
            0x0C        => ControllerEvent::EffectControl1,
            0x0D        => ControllerEvent::EffectControl2,
            0x10..=0x13 => ControllerEvent::GeneralPurposeControllers(byte - 0x10),
            0x20..=0x3F => ControllerEvent::LsbForControllers(byte - 0x20),
            0x40        => ControllerEvent::DamperPedal,
            0x41        => ControllerEvent::Portamento,
            0x42        => ControllerEvent::Sostenuto,
            0x43        => ControllerEvent::SoftPedal,
            0x44        => ControllerEvent::LegatoFootswitch,
            0x45        => ControllerEvent::Hold2,
            0x46..=0x4F => ControllerEvent::SoundController(byte - 0x46),
            0x50..=0x53 => ControllerEvent::GeneralPurposeControllers(byte - 0x50),
            0x54        => ControllerEvent::PortamentoControl,
            0x5B..=0x5F => ControllerEvent::EffectsDepth(byte - 0x5B),
            0x60        => ControllerEvent::DataIncrement,
            0x61        => ControllerEvent::DataDecrement,
            0x62        => ControllerEvent::NonRegisteredParameterNumberLsb,
            0x63        => ControllerEvent::NonRegisteredParameterNumberMsb,
            0x64        => ControllerEvent::RegisteredParameterNumberLsb,
            0x65        => ControllerEvent::RegisteredParameterNumberMsb,
            0x79..=0x7F => ControllerEvent::ModeMessages(byte - 0x79),
            _ => {
                let msg = format!("unknown MIDI controller message {:#04x}", byte);
                return Err(SynthError::new(&msg)) 
            }
        };
        Ok(controller_event)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChannelEventBody {
    NoteOff { note: u8, velocity: u8 },
    NoteOn { note: u8, velocity: u8},
    NoteAftertouch { note: u8, amount: u8 },
    Controller { controller_event: ControllerEvent, value: u8 },
    ProgramChange { program_number: u8 },
    ChannelAftertouch { amount: u8 },
    PitchBend { value: u16 }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MidiChannelEvent {
    channel: u8, // 4 bits in file
    event_body: ChannelEventBody
}

impl MidiChannelEvent {
    pub fn new(channel: u8, event_body: ChannelEventBody) -> Self {
        Self { channel, event_body }
    }

    pub fn parse<T: io::Read>(mut midi_stream: T, event_type: EventType, channel: u8) -> SynthResult<MidiChannelEvent> {
        let mut param1_byte: [u8; 1] = [0; 1];
        let mut param2_byte: [u8; 1] = [0; 1];
        read_with_eof_check!(midi_stream, &mut param1_byte);
        let param1 = param1_byte[0];
        let mut param2 = None;

        // We might need a second param to create the event
        match event_type {
            EventType::ProgramChange | EventType::ChannelAftertouch => {
                // These events have only one param. Don't read the second param...
            },
            _ => {
                // These events have two params. Read the second one.
                read_with_eof_check!(midi_stream, &mut param2_byte);
                param2 = Some(param2_byte[0]);
            }
        }

        #[cfg(feature = "verbose_midi")]
        {
            let param_string = format!("{}, {:?}", param1, param2);
            println!("Parsed MIDI channel event param bytes for {:?} event: {:?}", event_type, param_string);
        }

        let event_body = match Self::new_event_body(event_type, param1, param2) {
            Ok(inner_event) => inner_event,
            Err(err) => {
                let msg = format!("Failed to parse MIDI inner channel event: {}", err);
                return Err(SynthError::new(&msg));
            }
        };

        Ok(MidiChannelEvent { channel, event_body })
    }

    fn new_event_body(event_type: EventType, param1: u8, param2: Option<u8>) -> SynthResult<ChannelEventBody> {
        let event = match event_type {
            EventType::NoteOff => ChannelEventBody::NoteOff {
                note: param1, velocity: param2.expect("Expected this event to have a second param")
            },
            EventType::NoteOn => ChannelEventBody::NoteOn {
                note: param1, velocity: param2.expect("Expected this event to have a second param")
            },
            EventType::NoteAftertouch => ChannelEventBody::NoteAftertouch {
                note: param1, amount: param2.expect("Expected this event to have a second param")
            },
            EventType::Controller => {
                let controller_event = match ControllerEvent::from_byte(param1) {
                    Ok(controller_event) => controller_event,
                    Err(err) => {
                        let msg = format!("Failed to parse MIDI controller event: {}", err);
                        return Err(SynthError::new(&msg));
                    }
                };
                ChannelEventBody::Controller {
                    controller_event, value: param2.expect("Expected this event to have a second param")
                }
            },
            EventType::ProgramChange => ChannelEventBody::ProgramChange { program_number: param1 },
            EventType::ChannelAftertouch => ChannelEventBody::ChannelAftertouch { amount: param1 },
            EventType::PitchBend => {
                let lsb = (param1 as u16) & 0x007F;
                let msb = (param2.unwrap() as u16) & 0x007F;
                let value = lsb | (msb << 7);
                ChannelEventBody::PitchBend { value }
            }
            _ => {
                let event_byte = event_type.to_byte();
                let msg = format!("Unknown channel event type {:#03x}", event_byte);
                return Err(SynthError::new(&msg));
            }
        };
        Ok(event)
    }

    pub fn get_channel(&self) -> u8 {
        self.channel
    }

    pub fn get_inner_event(&self) -> &ChannelEventBody {
        &self.event_body
    }
}

// This is more or less completely tested by the parent module (midi::event)
