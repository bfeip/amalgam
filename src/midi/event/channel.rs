use super::super::error::*;
use super::MidiEventType;

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

pub struct MidiChannelEvent {
    delta_time: usize, // variable length in file
    event_type: MidiEventType, // 4 bits in file
    channel: u8, // 4 bits in file
    param1: u8,
    param2: u8
}

impl MidiChannelEvent {
    pub fn new(delta_time: usize, event_type: MidiEventType, channel: u8, param1: u8, param2: u8) -> Self {
        Self { delta_time, event_type, channel, param1, param2 }
    }
}