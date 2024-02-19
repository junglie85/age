use std::fmt::Display;

use age::Error;

#[derive(Debug)]
pub struct RendererError {
    msg: String,
    src: Option<Box<dyn std::error::Error>>,
}

impl RendererError {
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

impl Display for RendererError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.msg)
    }
}

impl std::error::Error for RendererError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.src.as_deref()
    }
}

impl From<&str> for RendererError {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<RendererError> for Error {
    fn from(value: RendererError) -> Self {
        Error::new("a renderer error occurred").with_source(value)
    }
}
