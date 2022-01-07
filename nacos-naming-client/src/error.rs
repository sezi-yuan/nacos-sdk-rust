use reqwest::StatusCode;
use serde_repr::*;
use thiserror::Error;

#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u32)]
pub enum RespCode {
    Ok = 10200,
    ResourceNotFound = 20404,
    NoNeedRetry = 21600
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Core(#[from] nacos_sdk_core::Error),
    #[error("{0}: {1}")]
    Fs(String, std::io::Error),
    #[error("no host to srv serviceInfo: {0}")]
    NoHostToService(String),
    #[error(transparent)]
    Net(#[from] reqwest::Error),
    
    #[error(transparent)]
    Serde(#[from] serde_json::Error),

    #[error("nacos server error; status: {0}, message: {1}")]
    NacosRemote(StatusCode, String),

    #[error("found invalid header value")]
    InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),
    #[error("{0}")]
    Custom(String),
    #[error("unknown error occurred")]
    Unknown
}

pub type Result<T> = std::result::Result<T, Error>;