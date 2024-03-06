use std::{error::Error, fmt::Display};

pub type AgeResult<T = ()> = Result<T, AgeError>;

#[derive(Debug)]
pub struct AgeError {
    message: String,
    source: Option<Box<dyn Error>>,
}

impl AgeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    pub fn with_source(self, err: impl Error + 'static) -> Self {
        Self {
            source: Some(Box::new(err)),
            ..self
        }
    }
}

impl Display for AgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.message)
    }
}

impl Error for AgeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_deref()
    }
}

impl From<&str> for AgeError {
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}
