use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct MockError {
    message: String,
}

impl fmt::Display for MockError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MockError: {}", self.message)
    }
}

impl Error for MockError {}
