/// Top level error returned by `main` and other high level functions
#[derive(Debug)]
pub struct SynthError {
    msg: String
}

impl SynthError {
    pub fn new(msg: &str) -> Self {
        let msg = msg.to_string();
        SynthError { msg }
    }
}

impl std::fmt::Display for SynthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    } 
}

impl std::error::Error for SynthError {}

pub type SynthResult<T> = Result<T, SynthError>;
