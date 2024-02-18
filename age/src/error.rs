use std::fmt::Display;

#[derive(Debug)]
pub struct Error {
    msg: String,
    src: Option<Box<dyn std::error::Error>>,
}

impl Error {
    pub fn new<S: Into<String>>(msg: S) -> Self {
        Self {
            msg: msg.into(),
            src: None,
        }
    }

    pub fn with_source<E: std::error::Error + 'static>(self, err: E) -> Self {
        Self {
            src: Some(Box::new(err)),
            ..self
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.msg)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.src.as_deref()
    }
}

impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}
