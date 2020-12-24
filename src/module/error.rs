/// Error type returned by modules
#[derive(Debug)]
pub struct ModuleError {
    msg: String
}

impl ModuleError {
    pub fn new(msg: &str) -> Self {
        let msg = msg.to_string();
        ModuleError { msg }
    }
}

impl std::fmt::Display for ModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    } 
}

impl std::error::Error for ModuleError {}

pub type ModuleResult<T> = Result<T, ModuleError>;
