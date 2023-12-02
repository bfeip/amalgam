use crate::error::{SynthError, SynthResult};

pub const FREQ_A:       f32 = 440.00; // A4
pub const FREQ_A_SHARP: f32 = 466.16;
pub const FREQ_B:       f32 = 493.88;
pub const FREQ_C:       f32 = 523.25; // C5
pub const FREQ_C_SHARP: f32 = 554.37;
pub const FREQ_D:       f32 = 587.33;
pub const FREQ_D_SHARP: f32 = 622.25;
pub const FREQ_E:       f32 = 659.25;
pub const FREQ_F:       f32 = 698.46;
pub const FREQ_F_SHARP: f32 = 739.99;
pub const FREQ_G:       f32 = 783.99;
pub const FREQ_G_SHARP: f32 = 830.61;

const MIDI_NOTE_BASE_OCTAVE: i8 = -1;
const MIDI_NOTE_BASE_TONE_OFFSET: u8 = 3; // numeric offset from A i.e. 3 is C

/// Represents the notes within an octave
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Hash)]
pub enum Tone {
    A,
    ASharp,
    B,
    C,
    CSharp,
    D,
    DSharp,
    E,
    F,
    FSharp,
    G,
    GSharp,
}

impl Tone {
    fn from_u8(n: u8) -> SynthResult<Self> {
        use Tone::*;
        let tone = match n {
            0  => A,
            1  => ASharp,
            2  => B,
            3  => C,
            4  => CSharp,
            5  => D,
            6  => DSharp,
            7  => E,
            8  => F,
            9  => FSharp,
            10 => G,
            11 => GSharp,
            _ => {
                let msg = format!("Tone index out of range: {}", n);
                return Err(SynthError::new(&msg));
            }
        };
        Ok(tone)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Note {
    pub octave: i8,
    pub tone: Tone
}

impl Note {
    pub const fn new(octave: i8, tone: Tone) -> Self{
        Self { octave, tone }
    }

    pub fn from_midi_note(midi_note: u8) -> Self {
        let octave = midi_note as i8 / 12 + MIDI_NOTE_BASE_OCTAVE;
        let tone_index = (midi_note + MIDI_NOTE_BASE_TONE_OFFSET) % 12;
        let tone = Tone::from_u8(tone_index).expect("Whoops! Tone out of range");
        Self { octave, tone }
    }

    pub fn to_freq(&self) -> f32 {
        let default_freq = match self.tone {
            Tone::A      => FREQ_A,
            Tone::ASharp => FREQ_A_SHARP,
            Tone::B      => FREQ_B,
            Tone::C      => FREQ_C,
            Tone::CSharp => FREQ_C_SHARP,
            Tone::D      => FREQ_D,
            Tone::DSharp => FREQ_D_SHARP,
            Tone::E      => FREQ_E,
            Tone::F      => FREQ_F,
            Tone::FSharp => FREQ_F_SHARP,
            Tone::G      => FREQ_G,
            Tone::GSharp => FREQ_G_SHARP,
        };
    
        let default_octave = if self.tone < Tone::C { 4 } else { 5 };
        let octave_shift = self.octave - default_octave;
    
        // E.g. A4 shifted down one octave is 440 * (2^-1) 
        let freq_shift_degree = 2_f32.powi(octave_shift as i32);
        default_freq * freq_shift_degree
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Hash)]
pub struct NoteInterval {
    pub note: Note,
    pub start: Option<usize>,
    pub end: Option<usize>
}

impl NoteInterval {
    pub const fn new(note: Note, start: Option<usize>, end: Option<usize>) -> Self {
        NoteInterval {note, start, end }
    }

    pub fn overlaps(&self, other: &NoteInterval) -> bool {
        let this_start = self.start.unwrap_or(0);
        let this_end = self.end.unwrap_or(usize::MAX);
        let other_start = other.start.unwrap_or(0);
        let other_end = other.end.unwrap_or(usize::MAX);

        if this_start >= other_start && this_start < other_end {
            // This starts within other
            return true;
        } 
        if this_end > other_start && this_end <= other_end {
            // This ends within other
            return true;
        }
        if other_start > this_start && other_start < this_end {
            // Other must be fully contained within this
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_from_midi_note() {
        let mut note = Note::from_midi_note(24);
        assert_eq!(note, Note{ octave: 1, tone: Tone::C });

        note = Note::from_midi_note(41);
        assert_eq!(note, Note{ octave: 2, tone: Tone::F });

        note = Note::from_midi_note(127);
        assert_eq!(note, Note{ octave: 9, tone: Tone::G });

        note = Note::from_midi_note(0);
        assert_eq!(note, Note{ octave: -1, tone: Tone::C });
    }


}