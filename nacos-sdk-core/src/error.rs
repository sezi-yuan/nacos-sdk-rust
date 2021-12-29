use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}: {1}")]
    Fs(String, std::io::Error),

    #[error(transparent)]
    Serde(#[from] serde_json::Error),

}

pub type Result<T> = std::result::Result<T, Error>;