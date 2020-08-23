use crate::prelude::*;

pub const FREQ_A:       Float = 440.00; // A4
pub const FREQ_A_SHARP: Float = 466.16;
pub const FREQ_B_FLAT:  Float = 466.16;
pub const FREQ_B:       Float = 493.88;
pub const FREQ_C:       Float = 523.25; // C5
pub const FREQ_C_SHARP: Float = 554.37;
pub const FREQ_D_FLAT:  Float = 554.37;
pub const FREQ_D:       Float = 587.33;
pub const FREQ_D_SHARP: Float = 622.25;
pub const FREQ_E_FLAT:  Float = 622.25;
pub const FREQ_E:       Float = 659.25;
pub const FREQ_F:       Float = 698.46;
pub const FREQ_F_SHARP: Float = 739.99;
pub const FREQ_G_FLAT:  Float = 739.99;
pub const FREQ_G:       Float = 783.99;
pub const FREQ_G_SHARP: Float = 830.61;
pub const FREQ_A_FLAT:  Float = 830.61;

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub enum Note {
    A,
    ASharp,
    BFlat,
    B,
    C,
    CSharp,
    DFlat,
    D,
    DSharp,
    EFlat,
    E,
    F,
    FSharp,
    GFlat,
    G,
    GSharp,
    AFlat
}

pub fn note_to_freq(note: Note, octave: Unsigned) -> Float {
    let default_freq = match note {
        Note::A      => FREQ_A,
        Note::ASharp => FREQ_A_SHARP,
        Note::BFlat  => FREQ_B_FLAT,
        Note::B      => FREQ_B,
        Note::C      => FREQ_C,
        Note::CSharp => FREQ_C_SHARP,
        Note::DFlat  => FREQ_D_FLAT,
        Note::D      => FREQ_D,
        Note::DSharp => FREQ_D_SHARP,
        Note::EFlat  => FREQ_E_FLAT,
        Note::E      => FREQ_E,
        Note::F      => FREQ_F,
        Note::FSharp => FREQ_F_SHARP,
        Note::GFlat  => FREQ_G_FLAT,
        Note::G      => FREQ_G,
        Note::GSharp => FREQ_G_SHARP,
        Note::AFlat  => FREQ_A_FLAT,
    };

    let default_octave = if note <= Note::C { 4 } else { 5 };
    let octave_shift = octave - default_octave;

    // E.g. A4 shifted down one octave is 440 * (2^-1) 
    let freq_shift_degree = (2 as Unsigned).pow(octave_shift);
    let freq = default_freq * freq_shift_degree as Float;
    freq
}