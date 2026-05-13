use thiserror::Error;

#[derive(Debug, Error)]
pub enum PptlintError {
    #[error("cannot open presentation '{path}': {message}")]
    Open { path: String, message: String },

    #[error("slide '{0}' not found in package")]
    SlideMissing(String),

    #[error("cannot parse slide: {0}")]
    ParseSlide(String),

    #[error("cannot read config '{path}': {source}")]
    ConfigRead {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("cannot parse config '{path}': {source}")]
    ConfigParse {
        path: String,
        #[source]
        source: toml::de::Error,
    },
}
