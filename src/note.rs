pub const FREQ_A:       f32 = 440.00; // A4
pub const FREQ_A_SHARP: f32 = 466.16;
pub const FREQ_B_FLAT:  f32 = 466.16;
pub const FREQ_B:       f32 = 493.88;
pub const FREQ_C:       f32 = 523.25; // C5
pub const FREQ_C_SHARP: f32 = 554.37;
pub const FREQ_D_FLAT:  f32 = 554.37;
pub const FREQ_D:       f32 = 587.33;
pub const FREQ_D_SHARP: f32 = 622.25;
pub const FREQ_E_FLAT:  f32 = 622.25;
pub const FREQ_E:       f32 = 659.25;
pub const FREQ_F:       f32 = 698.46;
pub const FREQ_F_SHARP: f32 = 739.99;
pub const FREQ_G_FLAT:  f32 = 739.99;
pub const FREQ_G:       f32 = 783.99;
pub const FREQ_G_SHARP: f32 = 830.61;
pub const FREQ_A_FLAT:  f32 = 830.61;

/// Represents the notes within an octave
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

/// Given a note and an octave this function will return a frequency
pub fn note_to_freq(note: Note, octave: u8) -> f32 {
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
    let freq_shift_degree = 2_u8.pow(octave_shift as u32);
    let freq = default_freq * freq_shift_degree as f32;
    freq
}