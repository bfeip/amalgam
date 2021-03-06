#[derive(Debug)]
pub struct MidiError {
    msg: String
}

impl MidiError {
    pub fn new(msg: &str) -> Self {
        let msg = msg.to_string();
        MidiError { msg }
    }
}

impl std::fmt::Display for MidiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    } 
}

impl std::error::Error for MidiError {}

pub type MidiResult<T> = Result<T, MidiError>;