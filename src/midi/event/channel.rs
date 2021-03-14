use std::io;

use super::super::error::*;
use super::MidiEventType;

#[derive(Debug, Clone, PartialEq)]
pub enum MidiControllerEvent {
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
    pub fn from_byte(byte: u8) -> MidiResult<Self> {
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

#[derive(Debug, Clone, PartialEq)]
pub enum MidiChannelEventBody {
    NoteOff { note: u8, velocity: u8 },
    NoteOn { note: u8, velocity: u8},
    NoteAftertouch { note: u8, amount: u8 },
    Controller { controller_event: MidiControllerEvent, value: u8 },
    ProgramChange { program_number: u8 },
    ChannelAftertouch { amount: u8 },
    PitchBend { value: u16 }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MidiChannelEvent {
    channel: u8, // 4 bits in file
    inner_event: MidiChannelEventBody
}

impl MidiChannelEvent {
    pub fn new(channel: u8, inner_event: MidiChannelEventBody) -> Self {
        Self { channel, inner_event }
    }

    pub fn parse<T: io::Read>(mut midi_stream: T, event_type: MidiEventType, channel: u8) -> MidiResult<MidiChannelEvent> {
        // Here I'm assuming that channel events that have only one param, like channel aftertouch
        // do actually have a second paramater that's simply unused
        let mut param_bytes: [u8; 2] = [0; 2];
        read_with_eof_check!(midi_stream, &mut param_bytes);
        let param1 = param_bytes[0];
        let param2 = param_bytes[1];

        let inner_event = match Self::new_inner_event(event_type, param1, param2) {
            Ok(inner_event) => inner_event,
            Err(err) => {
                let msg = format!("Failed to parse MIDI inner channel event: {}", err);
                return Err(MidiError::new(&msg));
            }
        };

        Ok(MidiChannelEvent { channel, inner_event })
    }

    fn new_inner_event(event_type: MidiEventType, param1: u8, param2: u8) -> MidiResult<MidiChannelEventBody> {
        let event = match event_type {
            MidiEventType::NoteOff => MidiChannelEventBody::NoteOff { note: param1, velocity: param2 },
            MidiEventType::NoteOn => MidiChannelEventBody::NoteOn { note: param1, velocity: param2 },
            MidiEventType::NoteAftertouch => MidiChannelEventBody::NoteAftertouch { note: param1, amount: param2 },
            MidiEventType::Controller => {
                let controller_event = match MidiControllerEvent::from_byte(param1) {
                    Ok(controller_event) => controller_event,
                    Err(err) => {
                        let msg = format!("Failed to parse MIDI controller event: {}", err);
                        return Err(MidiError::new(&msg));
                    }
                };
                MidiChannelEventBody::Controller { controller_event, value: param2 }
            },
            MidiEventType::ProgramChange => MidiChannelEventBody::ProgramChange { program_number: param1 },
            MidiEventType::ChannelAftertouch => MidiChannelEventBody::ChannelAftertouch { amount: param1 },
            MidiEventType::PitchBend => {
                let lsb = (param1 as u16) & 0x007F;
                let msb = (param2 as u16) & 0x007F;
                let value = lsb | (msb << 7);
                MidiChannelEventBody::PitchBend { value }
            }
            _ => {
                let event_byte = event_type.to_byte();
                let msg = format!("Unknown channel event type {:#03x}", event_byte);
                return Err(MidiError::new(&msg));
            }
        };
        Ok(event)
    }
}

// This is more or less completely tested by the parent module (midi::event)
