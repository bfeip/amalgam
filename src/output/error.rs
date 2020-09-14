#[derive(Debug)]
pub struct AudioOutputError {
    msg: String
}

impl AudioOutputError {
    pub fn new(msg: &str) -> Self {
        let msg = msg.to_string();
        AudioOutputError { msg }
    }
}

impl std::fmt::Display for AudioOutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    } 
}

impl std::error::Error for AudioOutputError {}

pub type AudioOutputResult<T> = Result<T, AudioOutputError>;
