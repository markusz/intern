use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot open presentation '{path}': {message}")]
    Open { path: String, message: String },

    #[error("slide '{0}' not found in package")]
    SlideMissing(String),

    #[error("cannot parse slide: {0}")]
    ParseSlide(String),

    #[error("cannot write presentation '{path}': {message}")]
    Write { path: String, message: String },
}
